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
| C16-AC3 | WAIVED_NOT_RUN | Hosted matrix, Windows rustc 1.88.0, macOS rustc 1.88.0, and clippy under rustc 1.88.0 stay NOT_RUN. On 2026-10-19 the user waived those results. They do not block C16 and they are not PASS. Linux 1.88.0 check, test, and fmt exited 0. See the 2026-10-19 waiver section. |
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

## 2026-10-17 continuation

Local canonical `just ci` on installed rustc/Cargo 1.99.0 exited 0. The log is
outside the repo, in the session scratch `c16-just-ci.txt` and `just-ci.txt`.
Counts in that run: CLI unit 191, `cli_contract` 13, `five_mode_contract` 8,
`provider_authority` 5, core 481 passed / 0 failed / 3 ignored, public API 2,
desktop 94 passed / 0 failed / 2 ignored. Clippy finished with warnings denied.
`check`, `test`, and `clippy` used `--target-dir dist/ci-target`. `fmt` was unchanged.

That local pass does not accept C16-AC3. Hosted run 39001489521 matches HEAD
`b0b5ce62f2994440743766ebcae3bae2a3feab59` and concluded failure. The working tree
after that run adds the Software Unix `LOCK_UN` drop, the Unix provider-fixture
canonical root, the justfile target directory, and the provider sentinel borrow
fix. Those edits are not on the hosted SHA. Declared MSRV 1.88.0 was not
downloaded or executed on this machine. The Software Unix audit-lock regression
has not run on Unix. C01 has not started.

## 2026-10-17 after libc 0.2.191

`just ci` exited 0 on rustc 1.99.0 (`b940084d7`, 2026-09-28). Log:
`C:\Users\lyh\AppData\Local\Temp\devsweep-just-ci-3.txt`. The recipe finished
with `ci complete` after fmt, locked workspace check, tests, and
`clippy -D warnings` using `--target-dir dist/ci-target`. The run reported
911 passed, 0 failed, 5 ignored. This does not accept C16-AC3. Declared MSRV
1.88.0 was not executed. Hosted CI was not re-run.

## 2026-10-17 after the software re-export format

`just ci` exited 0 again. Log: `C:\Users\lyh\AppData\Local\Temp\devsweep-just-ci-5.txt`.
Exit file: `devsweep-just-ci-5-exit.txt` (`exit=0`). The recipe ended with
`ci complete` after fmt, locked workspace check, tests, and `clippy -D warnings`
using `--target-dir dist/ci-target`. The summed test results are 911 passed,
0 failed, 5 ignored. The log does not print a rustc version. The check process reported
`rustc 1.99.0 (b940084d7 2026-09-28)`. This does not
accept C16-AC3. Declared MSRV 1.88.0 was not executed. Hosted CI was not re-run.
`desktop-gpui` is not a workspace member, so this gate did not build it.

## 2026-10-17 dependency audit

`cargo audit` exited 0. It reported 7 allowed warnings, starting with
`RUSTSEC-2024-0370` for `proc-macro-error` 1.0.4. Log:
`C:\Users\lyh\AppData\Local\Temp\devsweep-cargo-audit.txt`.
`cargo deny check --all-features` exited 2 on cargo-deny 0.20.2 because that
flag is not accepted. That run did not evaluate policy. `cargo deny check all`
then exited 0. The log ends with `advisories ok, bans ok, licenses ok, sources ok`
(`C:\Users\lyh\AppData\Local\Temp\devsweep-cargo-deny-all.txt`). This does not
accept C16-AC3.

## 2026-10-18 exact declared-MSRV execution on Linux

The declared core/CLI MSRV is `workspace.package.rust-version = "1.88.0"`. That
exact compiler had never been executed. The installed WSL Ubuntu-24.04 distro
carries `~/.rustup/toolchains/1.88.0-x86_64-unknown-linux-gnu`, but that install
has `rustc`/`rustfmt` and **no `cargo` binary**, so it cannot drive a build by
itself. `research/toolchain-decision.md` explicitly permits existing Linux WSL
tools as separate portability evidence without any new installation, so no
toolchain was downloaded.

Method: pin `RUSTC` to the 1.88.0 `rustc` and drive it with the installed
cargo 1.98.0. This was verified rather than assumed — a `cargo check -v` shows
the invoked compiler is literally
`/home/lyh/.rustup/toolchains/1.88.0-x86_64-unknown-linux-gnu/bin/rustc`.

| Gate (repo root, `--target-dir /tmp/dsw-msrv-verify`) | Exit | Result |
| --- | --- | --- |
| `cargo check -p devsweep-core -p devsweep-cli --locked --all-targets` | **0** | finished clean |
| `cargo test -p devsweep-core -p devsweep-cli --locked --all-targets` | **0** | 770 passed, 0 failed, 0 ignored |

Core lib alone: 521 passed, 0 failed. CLI integration targets: 13 + 8 + 2 + 2 +
12 + 4 passed.

Before the platform gating landed, the same command exited **101** with 6
failures in `devsweep-core` and 6 in `c01_shipped_parity`. Those were tests that
assumed Windows semantics (offline registry hives, volume identity, `C:\`
paths, Startup registry entries). They are now `#[cfg(windows)]`-gated, so they
still run on Windows and are compiled out on Linux. The gating changed no
assertion.

### What this does and does not prove

- **Does prove**: the declared 1.88.0 compiler accepts and passes the core and
  CLI code on Linux, and the hosted `Rust (ubuntu-latest)` and `MSRV
  (ubuntu-latest)` jobs can now run to completion instead of failing on
  Windows-only tests.
- **Does not prove**: 1.88.0 on Windows or macOS, or any hosted run. No
  `cargo fmt`/`clippy`/`deny`/`audit` gate was run under 1.88.0. Hosted CI
  remains NOT_RUN because `ci.yml` triggers only on `pull_request` and
  `workflow_dispatch`, and pushing is outside this task's authorization.

C16-AC3 therefore stays open on its matrix clause, but the "any declared
core/CLI MSRV is actually tested" clause now has a real, reproducible receipt
for Linux.

## 2026-10-19 MSRV toolchain coverage, item by item

The declared core/CLI MSRV is 1.88.0. Round 3 proved `check` and `test` pass
under it on Linux. This round established which of the other gate stages can and
cannot run on that exact compiler.

| Stage | Under rustc 1.88.0 | Evidence |
| --- | --- | --- |
| `cargo check -p devsweep-core -p devsweep-cli --locked --all-targets` | **exit 0** | 770-test run in round 3 |
| `cargo test  -p devsweep-core -p devsweep-cli --locked --all-targets` | **exit 0** | 770 passed, 0 failed |
| `cargo fmt --all -- --check` | **exit 0** | `rustfmt 1.8.0-stable (6b00bc3880 2025-06-23)`, driven by cargo 1.98.0 |
| `cargo clippy` | **cannot run** | the 1.88.0 toolchain ships **no clippy driver** (`bin/` has only `cargo-fmt`, `rust-gdb`, `rust-gdbgui`, `rust-lldb`, `rustc`, `rustdoc`, `rustfmt`). The only installed clippy is 0.1.95 for rustc 1.95, and pairing it with the 1.88 rustc fails with 331 errors (`E0220`, `E0432`, `E0514`) — a driver/compiler version mismatch, not a defect in this repository. |
| `cargo deny` / `cargo audit` | **not run** | separate tools, not installed for this toolchain |

So the "any declared core/CLI MSRV is actually tested" clause is satisfied for
**check, test and fmt** on Linux, and is **NOT_RUN for clippy** because the exact
toolchain has no clippy component and none was downloaded (the standing
constraint forbids installing Rust 1.88 tooling). This is recorded as a toolchain
gap, not waived.

Still NOT_RUN for C16-AC3 overall: the hosted Windows/macOS matrix, and 1.88.0 on
Windows or macOS. `ci.yml` triggers only on `pull_request` and
`workflow_dispatch`, and pushing is outside this task's authorization.

## 2026-10-19 user waiver

The user waived four NOT_RUN results. They stay NOT_RUN. They are not PASS.
They do not block C16.

- Hosted matrix, including the failed run 39001489521 on `b0b5ce62`.
- rustc 1.88.0 on Windows.
- rustc 1.88.0 on macOS.
- clippy under rustc 1.88.0. That toolchain has no clippy driver. None was installed.

Linux rustc 1.88.0 `cargo check`, `cargo test` (770 passed, 0 failed), and
`cargo fmt --all -- --check` stay exit 0. `cargo deny` and `cargo audit` were
not run under rustc 1.88.0.

On rustc 1.99.0, `just ci` exited 0 with `ci complete` (916 passed, 0 failed,
5 ignored). `git diff --stat 090249a HEAD -- crates desktop justfile Cargo.lock Cargo.toml .github`
was empty, so that workspace tree is the tree at `8b173e6`. `cargo audit` exited 0
(7 allowed warnings, no vulnerability failure). `cargo deny check all` exited 0
(`advisories ok, bans ok, licenses ok, sources ok`). These 1.99.0 receipts are
not an MSRV 1.88.0 pass. The C16-AC3 checkbox stays unchecked.
