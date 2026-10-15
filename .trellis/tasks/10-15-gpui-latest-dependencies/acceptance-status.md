# C16 implementation and independent review handoff

Date: 2026-10-15. Status: **implemented and independently reviewed; required
matrix evidence remains incomplete**. C16 is not complete or accepted by this
handoff. The main session owns final task disposition.

Selected Cargo.lock SHA-256:
`8b3db0d1771fa999d6f8b3d2df695aecae7752e8ef94ae3fa04529108f5cfefa`.
Final product snapshot: `evidence/check-source-receipt-final.json`, 26 files,
aggregate `a12e191faecba39fd8cec3d3af1069834f20d94bb3fd9affc884935a2ebd3485`.

## Acceptance mapping

| Criterion | Current result | Evidence / remaining work |
| --- | --- | --- |
| C16-AC1 | PASS | research/dependency-ledger.md: 32 Rust direct edges, 24 retained npm edges, 550 resolved packages, latest/selected versions, sources, compatibility exceptions and licenses. |
| C16-AC2 | PASS on tested platforms | Shared-core/CLI versions resolve and pass Windows 1.99/Linux 1.98 checks and semantic suites. GPUI candidate constraints are recorded; final native pin/build belongs to C09. |
| C16-AC3 | NOT_YET_ACCEPTED | Local isolated equivalents and final retained Tauri build pass. Canonical shared-cache execution still fails before core tests; exact declared MSRV, macOS and hosted runs remain unavailable. |
| C16-AC4 | Baseline identity and tested functional contracts PASS | research/baseline-receipt.json preserves clean baseline manifests/locks; reviewed digest encoding and shared schema/authority regressions pass. This is not runtime performance acceptance. |

## Reviewer fixes and final Windows evidence

The full review and scope decisions are in `research/check-review.md`.

- Keep `StatusSampler` stateless and `Default + Send + Sync`; retain PDH handles
  in call/worker-local `Rc` caches after windows-sys changes handles to pointers.
  A public API regression asserts the existing auto-trait guarantees. No unsafe
  Send/Sync or lint suppression was introduced.
- Correct the CLI test decoder's existing optional GPU/thermal outcome drift and
  add positive/negative supported-group cases. Production outcome stays unchanged.
- Explicitly unlock the Optimize Unix sidecar at owner drop; prove live-owner
  exclusion and replacement-owner safety with a deterministic duplicate-descriptor
  regression. Windows lock behavior stays unchanged.

Compiler: installed rustc/Cargo **1.99.0**, x86_64-pc-windows-msvc, as selected by
the user. Source identity above applies to every final reviewer receipt.

| Check | Result | Receipt in evidence/ |
| --- | --- | --- |
| Format | PASS | review-complete-fmt.log |
| Workspace all-targets check | PASS | review-complete-check.log |
| Workspace all-targets Clippy, warnings denied | PASS | review-complete-clippy.log |
| Workspace all-targets tests | PASS, 794 passed / 0 failed / 5 ignored | review-final-normal-temp-test.log and corresponding result.json |
| Retained Tauri no-bundle build | PASS, subprocess exit 0 | review-final-tauri-build.log/result.json and review-final-readback.json |

Final tests use explicit `--target-dir dist/c16-target-windows`, four test threads,
the normal external temporary directory, and no `CARGO_TARGET_DIR` or
`CARGO_BUILD_TARGET_DIR` environment override. Counts: 191 CLI unit, 26 CLI
integration, 481 core unit, 2 public API and 94 Tauri tests. The five ignored entries
retain their existing conditions: three native probes/benchmarks and two
child-process fixture entrypoints. No test was disabled.

## Retained failures and environment boundaries

- The first two canonical `just ci` attempts passed earlier stages but could not
  execute a missing core test binary in the shared mbx target (`os error 2`).
  A sandboxed final attempt stopped at `.cargo-build-lock` (`os error 5`).
  One authorized sandbox escalation passed format/check and CLI tests, then
  reproduced the missing core executable (`never executed`). See
  just-ci.log, just-ci-final.log, review-final-just-ci.log and
  review-escalated-just-ci.log/result.json. Canonical success is **not claimed**.
  No target link, cache policy or security setting was changed, and no further
  canonical retry was performed.
- Earlier sandboxed tests exposed OS error 5 replacing an isolated protection
  file, creating a settings directory and creating a private temporary registry
  tree. A temporary diagnostic was removed immediately after capturing the exact
  protection error; it is absent from the product diff. The same final source
  passes these tests outside the sandbox.
- The workspace-local TMP/TEMP workaround caused two Cargo metadata fixtures to
  be treated as undeclared workspace members. The exact Cargo error is preserved
  in review-workspace-temp-diagnostic.json. The final suite uses the normal
  external temporary directory, preserving fixture and product semantics.
- The genuine optional-probe test drift, the first new fixture's inconsistent
  process count, and the initial PDH Clippy error all remain in their original
  failing logs. Their corrected regressions and full suites passed afterwards.

## Compiler and cross-platform limits

- Core/CLI declared MSRV remains **1.88.0**. The user selected installed Windows
  1.99 for development/validation; no compiler was downloaded, installed or made
  a new global default. Current-compiler success does not prove exact MSRV.
- Final Linux WSL validation with installed Rust/Cargo **1.98.0** passes check,
  tests and warnings-denied Clippy: **674 passed, 0 failed, 0 ignored**, four test
  threads. See `research/linux-validation.md` for source/log hashes and commands.
- Earlier mixed Linux rustc 1.88/Cargo 1.98 receipts and repeated Optimize lock
  failures remain separately recorded. The final lock regression/full suite
  succeeds after the approved fix. The original holder PID was not traced;
  `research/linux-lock-findings.md` distinguishes the proven kernel mechanism.
- Exact Rust/Cargo 1.88 Windows/Linux, macOS and hosted execution remain
  **NOT_RUN**. The registered Linux 1.88 installation has no Cargo component.
  Existing CI jobs and the MSRV declaration remain intact. See
  `research/toolchain-decision.md` for the user instruction and its boundary.

## Retained desktop and supply chain

- Reused Node **22.23.2** / npm **10.9.8** receipts: generated-type check, lint,
  typecheck and build PASS; **471 Vitest + 5 Node tests PASS**. Four Vitest workers
  retain the original five-second deadlines. Reviewer fixes do not change React
  sources, package-locks, generated schemas or assets. Docs build also PASS.
- Earlier unsupported npm 12 and unbounded-worker timeout receipts remain
  separate failures, not supported-environment acceptance.
- Pre-review Tauri no-bundle build passed; it cannot prove the final Rust source.
  Its artifact and first harness quoting failure remain recorded separately.
  No installation, signing, publication or native behavior/performance claim.
- Final-source Tauri no-bundle build also PASS, 209.555 seconds; artifact SHA-256
  `df36cc169970fa0b137eb87e4a7442af331c60fad9a2b93a6d46882439e2320f`,
  15,027,200 bytes. After the complete UTF-8 log and success JSON were saved,
  printing a checkmark through the outer wrapper's GBK stdout failed. The
  separately recorded read-back confirms build exit 0, unchanged source and the
  artifact hash. No successful build was rerun to erase this display failure.
- Final `cargo audit --json`: exit 0, zero vulnerability-class entries. Advisory
  database commit `edb58f5974b83eb8302332821cb6d389777aa351`, 1329 advisories.
  `lru` 0.18.5 fixes RUSTSEC-2026-0253; six unmaintained and one unsound legacy
  notices remain explicitly documented. No claim of a warning-free audit.
- Final `cargo deny --all-features check`: exit 0; existing advisories, bans,
  licenses and sources policy passes without new ignores or exceptions.

No real user cleanup, software removal or host repair was executed. Isolated
fixture tests do not constitute GPUI, uninstaller or performance acceptance.
