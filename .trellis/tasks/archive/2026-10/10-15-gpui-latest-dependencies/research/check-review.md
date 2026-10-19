# C16 independent review

Date: 2026-10-15. Reviewer: dispatched `trellis-check`.
Status: product review and local fixes complete; final Linux and Windows isolated
verification and the retained Tauri build passed. C16 is not accepted or complete
by this report because required matrix evidence remains incomplete.

## Scope and identity

Reviewed the entire C16 tracked product diff and the untracked private module
`crates/devsweep-core/src/digest_encoding.rs`. Planning directories are not treated
as application changes. Read the native hook output, child PRD/design/plan,
curated contracts, applicable layer indexes, persistence/error contracts, and
the full quality sections for CLI output, PDH and CI.

Base commit: `25437d102ab7355198b3ef2647c2cbced740b3b2`.
Cargo.lock SHA-256:
`8b3db0d1771fa999d6f8b3d2df695aecae7752e8ef94ae3fa04529108f5cfefa`.
Post-fix product files and their individual hashes are recorded in
`../evidence/check-source-receipt-final.json`; aggregate:
`a12e191faecba39fd8cec3d3af1069834f20d94bb3fd9affc884935a2ebd3485`.
The earlier sampler-only snapshot remains in check-source-receipt.json. The final
receipt also includes the main session's code-map update.

## Findings (fixed)

- Files: `crates/devsweep-core/src/status/sampler.rs`,
  `crates/devsweep-core/tests/public_api.rs`.
  Issue: windows-sys 0.61 makes PDH query/counter handles raw pointers.
  `Arc<PdhProbe>` therefore ceased to be Send/Sync, broke the existing public
  `StatusSampler` auto-traits, and failed the required Clippy gate. Replacing
  Arc with Rc alone would leave that public regression in place.
  Fix: following the main session's explicit integration decision, retain a
  privately constructible stateless `StatusSampler` with its existing Default
  and snapshot API. Its old private cache was never mutated: snapshot cloned
  an always-default value, while live already created its own cache inside the
  worker. Snapshot now creates a local cache directly. Only that local cache
  shares its query through Rc. Remove the now-unused private Clone derivation.
  A separate external-crate assertion requires Default + Send + Sync for the
  public sampler. No unsafe Send/Sync assertion, lint suppression, changed
  counter math, changed cadence, or new worker was introduced.
  Rationale is also preserved in `change-boundary.md`.

- File: `crates/devsweep-cli/tests/cli_contract.rs`.
  Issue: the independent test decoder still counted optional GPU/thermal failures
  as a degraded snapshot, contradicting the existing core implementation and
  PDH spec. A real snapshot with all six supported groups available and unavailable
  optional probes exposed this old drift.
  Fix: exclude those two optional probes from the test's outcome calculation.
  Add deterministic success/partial positive and negative cases that still reject
  a successful envelope when the supported CPU group is unavailable. The first
  new fixture retained an inconsistent process count; this genuine test-authoring
  failure is retained, then corrected to one enumerated/returned process with
  no truncation or exhausted budget. No schema validation was removed and no
  production outcome changed. The corrected regression passes 1/1.

- File: `crates/devsweep-core/src/optimize/audit.rs`.
  Issue: the existing Unix sidecar released its flock only by closing its File.
  A concurrent fork can retain the same open file description until exec and
  prevent the next operation from acquiring the lock after the owner returns.
  Repeated full Linux runs failed different Optimize tests at this boundary.
  [Independent research](linux-lock-findings.md) proves the kernel mechanism with
  ten close-only failures and ten explicit-unlock successes using pipe barriers;
  the exact child PID from the original product failures was not traced.
  Fix approved by the main session: a Unix-only Drop explicitly calls LOCK_UN
  while the guard owns its descriptor. Acquisition, conflict refusal, sidecar
  identity and all Windows behavior stay unchanged. The regression duplicates
  the same open file description via File::try_clone, verifies a live owner
  still blocks an independent opener, verifies owner drop immediately permits
  reacquisition while the duplicate remains alive, and verifies dropping that
  old duplicate cannot release the replacement owner's lock. No retry, sleep,
  serialized test workaround, lock-file unlink, or public helper was added.

## Reviewed without additional defects

- All 32 direct Rust edges resolve to the ledger selections. Selected latest
  core/CLI versions agree with retained official registry snapshots; older
  windows-collections/future 0.3.2 are documented compatibility exceptions to
  0.100.0 because windows 0.62.2 uses windows-core 0.62. No floating Git or wildcard
  source was added. The complete final metadata and lock have the same 550
  package/version pairs.
- The digest helper emits two lowercase hex characters per byte. Empty/abc
  SHA-256 vectors and leading-zero encoding are tested. Every migrated caller
  retains its hash preimage, prefix, ordering, normalization and serialization
  boundary; no JSON field, algorithm, ID scheme or authority schema changes.
- WinRT collection and AsyncStatus moves retain the same MTA boundary, explicit
  current-user identity, partial enumeration handling, cancellation, removal
  polling and post-dispatch unknown states. The error-constructor rename does
  not change the caller's existing rejection handling.
- Typed PDH handles retain null checks, query ownership and exactly-once close;
  counters remain query-owned. The approved local-cache correction keeps the
  handles within the sampling call/worker. Fixture tests are not proof of actual
  sensor availability or native uninstaller behavior.
- Non-Windows and MSRV CI explicitly select core/CLI while Windows retains the
  workspace gate. English and Chinese CI descriptions agree. The current 1.88
  declaration remains unchanged. No GPUI pin or compiler-compatibility result is
  inferred from a candidate audit; C09 owns that native gate.
- The old Tauri baseline is identified by clean commit and copied manifests/locks.
  Shared dependency updates do not replace historical benchmark evidence.
  cargo-audit's final JSON has zero vulnerability-class entries and retains six
  unmaintained plus one unsound notice. cargo-deny passes the existing policy;
  no ignore or license exception was added. Legacy GTK/glib warnings are recorded
  rather than described as a clean security result.

## Findings (not fixed)

1. **Required matrix evidence remains incomplete.** Exact Windows Cargo/Rust 1.88,
   macOS execution and hosted jobs have not been established by the reviewed
   receipts. Final Linux validation uses installed Rust/Cargo 1.98; earlier mixed
   Rust 1.88/Cargo 1.98 evidence remains separately identified. Do not mark C16-AC3
   complete from available modern-compiler or mixed-toolchain evidence.
   The user subsequently selected installed Rust 1.99 for Windows validation;
   no 1.88 installation is authorized or attempted. This does not silently change
   the still-declared minimum compiler. See toolchain-decision.md. Toolchain
   availability and matrix acceptance belong to the main session.
2. **Canonical gate environment failure.** The first two retained `just ci` attempts
   stopped because the shared-cache core test executable was missing before
   execution. Isolated explicit-target equivalents establish separate evidence,
   not a successful canonical invocation. A single final canonical attempt with
   workspace-local TMP/TEMP then failed opening target/debug/.cargo-build-lock
   with OS error 5 before compilation; review-final-just-ci.log retains it.
   A subsequent single `require_escalated` canonical invocation passed format,
   check and CLI tests, then again could not execute the missing shared-target
   core test binary (OS error 2, never executed). Its receipt is
   review-escalated-just-ci.log/result.json. This is not a completed canonical
   gate; no further retries or cache/link/security changes were made.
3. **Related Software lock pattern outside the approved repair.** Research found
   the same close-only Unix release in software/audit.rs. No cited suite
   failure identified that path; the main session bounded the repair to Optimize.
   Recommend reviewing the Software sibling with its owning task rather than
   expanding this change into an audit-system refactor.

## Validation harness failures retained

- The first post-sampler Windows run and its exact protection-test rerun failed
  with protection_store_unavailable. A temporary test-only diagnostic identified
  `Io { stage: replace_protection_store, OS error 5 }` from fs::rename under the
  Windows sandbox's AC/Temp directory. The diagnostic was immediately removed;
  protect.rs has no final diff. The unchanged exact test passed using process-local
  TMP/TEMP under dist/c16/review-temp. These roots are owned test state, not user
  protection stores. No permission or security setting was changed.
- The next full suite exposed the optional-probe decoder drift described above.
  A later run also caught the newly added fixture's process-count inconsistency;
  both failures remain in their original logs. The corrected fixture was tested
  separately before repeating full suites.
- The final sandboxed workspace test run passed CLI/integration tests, then had
  four core failures: directory creation and a private HKCU test key both returned
  OS error 5, while two Cargo fixture tests received local-fallback targets instead
  of the expected valid metadata-backed action. These are preserved in
  review-complete-test.log. The follow-up isolated-target suite uses the same
  source, four threads and workspace TMP/TEMP under `require_escalated` cleared
  both OS error 5 failures but retained the two Cargo fixture failures. A separate
  diagnostic then captured Cargo's precise error: these temporary packages below
  the repo root believe they belong to its workspace, but are not members. See
  review-workspace-temp-diagnostic.json. The final suite with the normal external
  temporary directory passed unchanged. No product changes, workspace exclusions,
  fixture weakening or policy relaxation were made in response to these failures.

## Verification

Post-fix reviewer checks:

- `cargo fmt --all -- --check`: PASS.
- `git diff --check`: PASS.
- Windows workspace check/Clippy: PASS on the final source.
- Corrected optional-probe regression: PASS 1/1.
- Windows full workspace tests: PASS on the final source with installed Rust/Cargo
  1.99, explicit isolated target, normal external temporary directory, four threads
  and sandbox escalation: 794 passed, zero failed, five existing ignored entries
  (three native probes/benchmarks and two child-process fixture entrypoints).
  Receipt: review-final-normal-temp-test-result.json/log.
- Linux Rust/Cargo 1.98 check/test/Clippy: PASS on the final source; 674 tests,
  zero failures/ignored at four threads, including the deterministic lock and
  corrected status regressions. Exact commands and receipts: linux-validation.md.
- Main-owned updated persistence, PDH, directory and CI specs were reviewed and
  accurately scope the changes and unverified MSRV/platform boundaries.
- Final-source Tauri no-bundle build: PASS, subprocess exit 0; Node 22/npm 10,
  explicit isolated target, 209.555 seconds. Artifact SHA-256
  `df36cc169970fa0b137eb87e4a7442af331c60fad9a2b93a6d46882439e2320f`,
  15,027,200 bytes. See review-final-tauri-build-result.json and
  review-final-readback.json. The outer display wrapper failed only after saving
  the complete log/result because GBK stdout could not print a checkmark. The
  artifact hash, build exit code, log and unchanged source were read back; the
  successful build was not repeated. This is a build, not native behavior proof.

Reused pre-fix receipts (not represented as post-fix acceptance):

- Windows isolated workspace check PASS; tests 792 passed, 5 ignored. The original
  Clippy failure is retained in `isolated-clippy.log`.
- Node 22.23.2/npm 10.9.8 generated-type check, lint, typecheck and build PASS;
  unchanged frontend has 471 Vitest tests passing with four workers and original
  deadlines, plus five Node tests. Prior timeout/unsupported-engine attempts remain
  recorded separately. The sampler-only correction does not change frontend files,
  generated schemas, package-locks or assets.
- cargo-audit and cargo-deny final receipts apply to the unchanged final lock.
- The completed pre-review Tauri no-bundle receipt remains separate from the
  final-source build above. No native behavior or performance is claimed.

No commit, archive, installation, actual user cleanup, software removal or host
repair was performed by this review. The main session owns spec synchronization
and final acceptance recording.
