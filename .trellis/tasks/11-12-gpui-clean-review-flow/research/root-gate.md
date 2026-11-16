# Root workspace gate

Date: 2026-11-15. Baseline: `dev` / `faf9dc6`. The approved AC6 data-source
boundary and Command review-only boundary were refreshed from the task artifacts.
This receipt covers the root workspace only; `desktop-gpui/` is standalone and
was being edited by another agent. Root crates, manifests, lockfile and justfile
had no diff. Only this research note is owned by this gate review.

## Initial canonical run

Executed from `D:\Documents\Code\Rust\Exp\devsweep`:

```powershell
$env:RUST_TEST_THREADS='4'; just ci
```

The environment setting applies four test threads, matching the existing
`test-stable` recipe, while retaining the canonical `ci` dependency chain.

| Stage / exact underlying command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo check --workspace --locked --all-targets --target-dir dist/ci-target` | Pass; finished in 0.63s |
| `cargo test --workspace --locked --all-targets --target-dir dist/ci-target` | Fail; one core unit test failed |
| `cargo clippy --workspace --locked --all-targets --target-dir dist/ci-target -- -D warnings` | Not reached because `test` failed |

`just ci` exited **1**. It must not be reported as passed. Cargo was released and
the main agent notified immediately when the command ended. The full stdout
was automatically retained by RTK at
`C:\Users\lyh\AppData\Local\rtk\tee\1794790760_just_ci.log` (940 lines;
last write 2026-11-15 19:59:20 -05:00).

| Completed test binary / suite | Passed | Failed | Ignored | Reported test time |
| --- | ---: | ---: | ---: | ---: |
| CLI `src/lib.rs` | 208 | 0 | 0 | 2.68s |
| CLI `src/main.rs` | 0 | 0 | 0 | 0.00s |
| CLI `src/bin/process_fixture.rs` | 0 | 0 | 0 | 0.00s |
| `c01_shipped_parity` | 8 | 0 | 0 | 2.18s |
| `c05_software_maintenance` | 2 | 0 | 0 | 7.68s |
| `c07_analyze_parity` | 2 | 0 | 0 | 3.16s |
| `c15_ai_cleanup` | 12 | 0 | 0 | 3.21s |
| `cli_contract` | 15 | 0 | 0 | 3.90s |
| `five_mode_contract` | 8 | 0 | 0 | 2.85s |
| `provider_authority` | 5 | 0 | 0 | 20.73s |
| Core `src/lib.rs` | 600 | 1 | 3 | 70.72s |
| **Total reached** | **860** | **1** | **3** | |

The core test binary registered 604 tests. The three existing ignored tests are
the native Settings probe and two read-only MSIX benchmarks. Later targets, if
any, were not completed after the failing core binary; this is not a full test
suite pass. `provider_authority` passed all five tests, so no provider timeout
rerun was needed.

## Failure and initial disposition

```text
process::tests::endless_stderr_is_capped_with_truncation_mark
crates/devsweep-core/src/process/mod.rs:524:9
assertion failed: result.output.stderr_truncated
```

The test reported `stderr_truncated=false`. The initial output does not establish
the cause or whether the failure repeats. It is not classified as a known
baseline failure. No source or test was changed. The main agent was notified
before further investigation; a subsequent instruction authorizes only source
inspection and a serial isolated rerun of this exact test.

## Authorized isolated rerun and diagnosis

After the initial receipt and failure notification, the main agent explicitly
authorized inspection and a serial rerun of only this case. The existing fixture
binary was checked before execution so the test helper could not fall back to
launching Cargo (`process/mod.rs:328`). Exact command from the root:

```powershell
if (-not (Test-Path -LiteralPath .\dist\ci-target\debug\process_fixture.exe -PathType Leaf)) { throw 'Existing process_fixture.exe is required; refusing the test helper Cargo fallback' }; & .\dist\ci-target\debug\deps\devsweep_core-a1716d35389e43cb.exe process::tests::endless_stderr_is_capped_with_truncation_mark --exact --nocapture --test-threads=1
```

Result: **exit 0; 1 passed, 0 failed, 0 ignored, 603 filtered out; 0.43s**.
This was one isolated rerun, using existing binaries without Cargo or compilation.
It did not reproduce the failure and does not turn the failed full gate into a
pass or establish a known baseline failure.

Relevant source evidence:

- `crates/devsweep-core/src/process/mod.rs:509` sets a 32 KiB stream cap and a
  400 ms timeout. The first assertion verifies Timeout (`:523`); the failed
  assertion then assumes enough stderr arrived to exceed that cap (`:524`).
- The timeout deadline is created before neutral-directory preparation and
  process spawn (`:146`, `:151`, `:175`, `:286`). Capture readers start after
  spawn and Job Object attachment (`:187`, `:201`). Startup and scheduling
  delays therefore consume the same 400 ms budget as output production.
- `crates/devsweep-cli/src/bin/process_fixture.rs:73` writes stderr in 4 KiB
  chunks indefinitely, without a readiness/minimum-byte handshake. A correct
  timeout can occur before more than 32 KiB has been captured; a false
  truncation flag in that situation is correct, despite this test's expectation.
- `process/capture.rs:56` reads bounded chunks; `ByteRing::push` (`:88`) sets
  truncation when its cap is exceeded. A second possible timing path is
  `collect_capture` (`:119`): it can choose an empty default after its grace
  period before joining the reader (`:130`, `:138`). The failed run did not
  print byte totals or reader timing, so these possibilities cannot be
  distinguished from that output alone.

Assessment: the source exposes a timeout/output-volume race consistent with the
observed failure and isolated pass. The cause is **not proven** from one rerun;
there is no evidence here that the output cap itself is broken. No product or
test source was changed, and this process-runner code has no diff in the Clean
review task. If it recurs, the smallest useful follow-up is failure diagnostics
showing retained/total bytes and capture completion, then a fixture arrangement
that separates guaranteed cap overflow from the independent timeout assertion.
Do not weaken production deadlines or cap assertions merely to obtain a green
run. Such maintenance is outside this gate-only write scope.

A complete `just ci` still needs to be rerun later, serially with GPUI gates.
No second heavy gate was started. Main was notified of the isolated result and
the outstanding full-gate status.

## Final full rerun after GPUI review

On 2026-11-15, after `just gpui` completed successfully, ran the canonical root
gate again, with no concurrent Cargo gate and no root code/test changes:

```powershell
$env:RUST_TEST_THREADS='4'; just ci
```

**Exit 0: format, type-check, tests and Clippy `-D warnings` all passed.**
This complete run supersedes the initial failure as the final gate status,
without erasing its evidence or declaring its cause proven. The previously
failing `process::tests::endless_stderr_is_capped_with_truncation_mark` passed
in the full run. `provider_authority` passed all five tests; no additional
isolated rerun was needed.

| Suite | Passed | Failed | Ignored | Reported test time |
| --- | ---: | ---: | ---: | ---: |
| CLI `src/lib.rs` | 208 | 0 | 0 | 1.55s |
| CLI `src/main.rs` | 0 | 0 | 0 | 0.00s |
| CLI `src/bin/process_fixture.rs` | 0 | 0 | 0 | 0.00s |
| `c01_shipped_parity` | 8 | 0 | 0 | 0.93s |
| `c05_software_maintenance` | 2 | 0 | 0 | 3.92s |
| `c07_analyze_parity` | 2 | 0 | 0 | 1.52s |
| `c15_ai_cleanup` | 12 | 0 | 0 | 2.51s |
| `cli_contract` | 15 | 0 | 0 | 3.37s |
| `five_mode_contract` | 8 | 0 | 0 | 3.59s |
| `provider_authority` | 5 | 0 | 0 | 16.93s |
| Core `src/lib.rs` | 601 | 0 | 3 | 47.98s |
| Core `src/bin/windows_action_helper.rs` | 0 | 0 | 0 | 0.00s |
| Core `public_api` | 2 | 0 | 0 | 0.00s |
| **Total** | **863** | **0** | **3** | |

There were 866 registered tests in the reached complete run; the same three
existing ignored native/benchmark tests remain ignored. Final check took 0.28s;
Clippy took 0.53s. Full stdout:
`C:\Users\lyh\AppData\Local\rtk\tee\1794793162_just_ci.log`.
Main was notified immediately that Cargo was free when the command ended.

## Scope and limitations of the initial gate investigation

No GPUI gate, Python command, manual host scan, cleanup, uninstall, update,
startup action or native automation was invoked. The authorized root tests ran
their existing fixtures. Cargo's existing mbx wrapper emitted automatic cache/GC
diagnostics; no explicit cleanup command was issued. No commit, push or archive
was performed. GPUI implementation and native acceptance remain separate work.
