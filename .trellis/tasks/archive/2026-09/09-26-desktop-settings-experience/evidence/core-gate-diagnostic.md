# Core Gate Diagnostic

Date: 2026-09-27, America/Chicago. Read-only filesystem and event-log snapshot
completed at 22:03. Scope: four parent gate logs, target-directory resolution,
build command/environment lookup, and the relevant core test helpers.

## Conclusion

The cause remains undetermined. The evidence shows two distinct failures:

1. Three requested logs report a missing test executable before the core
   harness starts. The core test body cannot remove its executable during an
   attempt in which that executable never starts.
2. One requested log records an actual core test process ending with
   `0xc0000005, STATUS_ACCESS_VIOLATION`. The log has no crash stack or named
   failing test. The last completed test does not identify the faulting test.

The target link resolves to an accessible directory. Both reported core
executables were absent from the logical path and the physical cache path at
inspection time. The link observation alone does not identify which process
removed, replaced, or failed to retain an executable.

## Gate evidence

Paths in this section are relative to this parent task's `evidence/`.

| Evidence | Observed result |
| --- | --- |
| `ci-first-attempt.log:278-285` | Cargo announces `devsweep_core-0942b4e2b82cca0a.exe`, then reports `(never executed)` and `os error 2`. |
| `ci.log:262-269` | The same executable name again fails before execution with `os error 2`. |
| `preferences-final.log:1-10` | Compilation finishes, but `devsweep_core-2ca6c9bc5b0fca68.exe desktop_preferences --test-threads=1` never executes; `os error 2`. |
| `core-native-cargo.log:3-5, 212-216` | The `2ca6c9bc5b0fca68` executable starts a 385-test harness, then terminates with `STATUS_ACCESS_VIOLATION`. There is no successful suite summary. |

The access-violation log contains 203 completed `ok` records and one ignored
record. All 14 desktop-preference tests have `ok` records at lines 16-26,
36-37, and 46. Those partial results do not close the failed core gate.

The additional existing `core-serial-diagnostic.log:1-10` also reports
`(never executed)` for the `2ca6c9bc5b0fca68` executable with
`--test-threads=1 --nocapture`. That attempt supplies no evidence about
serial test-body behavior.

## Build environment and target directory

The repository `target` entry is a `SymbolicLink` to the following path
(Windows separators are normalized here for readability):

```text
C:/Users/lyh/AppData/Local/mbx/targets/v1/32e83a3e1a515af28254447a8642a7e835abfd3da70c33f4e746fbfe85e1709f
```

The physical target directory, `debug`, and `debug/deps` were ordinary
accessible directories. Neither of these files existed through either path:

- `debug/deps/devsweep_core-0942b4e2b82cca0a.exe`
- `debug/deps/devsweep_core-2ca6c9bc5b0fca68.exe`

Matching `.d` and `.pdb` files existed. The two PDB files were 75,354,112 and
75,370,496 bytes, with last-write times 21:45:01 and 21:45:58 respectively.
Those files show retained build outputs; they do not prove when an executable
became absent. `target/debug/process_fixture.exe` existed at inspection time
(292,864 bytes; last written 21:45:02). Its presence during the earlier
access-violation run has not been established.

Read-only command lookup returned:

- First PATH Cargo: `C:/Users/lyh/AppData/Local/mbx/bin/cargo.exe`.
- Rustup proxy: `C:/Users/lyh/.cargo/bin/cargo.exe`.
- `rustup which cargo`: `C:/Users/lyh/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe`.

The parent reports using the toolchain Cargo directly. A direct invocation
without a different target directory still writes through the same
repository target link. That invocation separates the Cargo entry point,
but does not separate the output storage.

In the diagnostic shell, `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR`,
`CARGO_HOME`, `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`, `RUSTC_WRAPPER`,
`RUSTC_WORKSPACE_WRAPPER`, `RUSTFLAGS`, and `CARGO_ENCODED_RUSTFLAGS` were unset.
No Cargo config file existed at the inspected repository/home paths or the
ancestor `.cargo/config.toml` paths. These observations describe the diagnostic
shell; they do not reconstruct all child environments from earlier runs.

The gate runner delegates to `just` without a target override
(`evidence/verify/run-gate.ps1:23-27`). The canonical test command is
`cargo test --workspace --locked --all-targets` (`justfile:17-18`).

## Test helper and temporary-directory review

| Source | Relevant behavior and limit |
| --- | --- |
| `crates/devsweep-core/src/process/mod.rs:307-359` | `process_fixture_exe` derives the sibling fixture path from `current_exe`. A process-local `Once` builds the CLI fixture only when that path is absent. The fallback uses `CARGO` or PATH Cargo, the explicit workspace directory, and `--target-dir` derived from the current executable. The fallback is a build, with no clean command. The existing log cannot establish whether the fallback ran. |
| `crates/devsweep-core/src/process/mod.rs:280-305` | Neutral process directories are named `devsweep-proc-<pid>-<nonce>` beneath `env::temp_dir()`. Cleanup receives only that allocated directory. Explicit working directories return no cleanup directory. The inspected TEMP/TMP values point to `C:/Users/lyh/AppData/Local/Temp`, separate from the target cache. |
| `crates/devsweep-core/src/process/mod.rs:551-569` | One existing test changes the process-wide working directory, restores it, and deletes its own temporary project. That shared current-directory mutation is a concurrency variable for further investigation, but no evidence connects it to either reported failure. It cannot explain a failure before the harness starts. |
| `crates/devsweep-core/src/execution/mod.rs:1742-1798, 1820-1874, 2501-2573` | Self-clean guard tests use the current executable directory with recording command/trash runners. The recording command runner records requests and returns a result; it does not invoke the `cargo clean` text carried by the test plan. |
| `crates/devsweep-core/src/execution/safety/mod.rs:466-484` | The live guard rejects containment of the running executable and fails closed when executable lookup fails. |
| `crates/devsweep-core/src/desktop_preferences/tests.rs:20-23, 207-209, 464-480` | Preference tests place files beneath retained `tempfile` roots. The concurrent test joins its worker handles. The preference tests do not use `current_exe`, the process fixture helper, or a Cargo command. |
| `crates/devsweep-cli/src/bin/process_fixture.rs:15-86` | Fixture modes implement child hangs, bounded-test output, exit, PID-file output, and working-directory output. No mode deletes the core executable or target directory. |

The review found no supported path from these helpers to deletion of the
core test executable. The review does not prove that every native/unsafe
operation in the 385-test suite is free of defects.

## Crash evidence availability

A read-only Application event-log query for IDs 1000/1001 between
2026-09-27 21:25 and 21:40 local time returned eight events. None matched
`devsweep_core`. The existing local `CrashDumps` directory contained no
matching core dump. No event-log or dump settings were changed.

The access-violation log has no faulting module, instruction address, or
thread stack. A product-code cause cannot be established from the exit code
or the last completed-test line alone.

## Smallest verifiable next step

The parent should run one controlled attempt after other Cargo work finishes.
Use the verified toolchain Cargo path and a new, absolute, ordinary directory
outside the existing target link/cache. Pass that directory with the command's
`--target-dir`; preserve the existing link, cache, and global configuration.

1. Build only the core test executable with `test --locked -p devsweep-core
   --lib --no-run --message-format=json --target-dir <new-directory>`. Record
   the exact command/environment and Cargo's compiler-artifact `executable`
   field. Check file existence, size, and SHA-256 immediately after Cargo exits.
2. If present, invoke that exact absolute executable with `--list`. Record
   its exit code and recheck the same file. No test body runs in this step.
3. If listing succeeds, invoke the same executable with
   `desktop_preferences --test-threads=1 --nocapture`. That filter does not
   use the fixture-building helper. Record result and final file identity.

Interpret the boundaries separately:

- An executable absent immediately after `--no-run` needs build/output file
  tracing. Running additional test bodies cannot identify that cause.
- Success in the isolated directory establishes that the focused test can
  build and run there. It does not identify the actor affecting the old
  directory, and does not close `just ci`.
- A crash during `--list` occurs before test bodies. A crash only after an
  individual test starts needs a recorded failing test and native stack.
- If the filtered tests pass, the parent can separately attempt a serial
  full-core run from the same stable executable. Ensure the CLI fixture is
  already built in that isolated target before exercising process tests, so
  the conditional nested-Cargo path is not an uncontrolled variable.

These steps are proposals. This diagnostic ran no Cargo command, test binary,
cleanup, global configuration change, or security-setting change.

## Evidence hashes

SHA-256 values identify the logs inspected for this report.

| Log | SHA-256 |
| --- | --- |
| `ci.log` | `28e5d1806d581506f26d806ac988d52d31c8d6f4fb81f8d93b722dd1c58337f4` |
| `ci-first-attempt.log` | `c199dec94a043614ed0322e6403501888070cebcabd64c500aeea06ae3325618` |
| `core-native-cargo.log` | `884290502109720f302029fc845f24bc4bf0b8e5a148d30b17fd9a3b184820f1` |
| `preferences-final.log` | `f64e34bc74c9a56b427b5b4e72109f9311a540dba3d35d19c56095e2d5c1df1d` |

Only this diagnostic report was written. No product or task-state file changed.

## Isolated build result supplied by the parent

The parent executed the proposed isolated build from 2026-09-28 03:04:29 to
03:04:58 UTC. `evidence/core-isolated.json` records the real toolchain Cargo,
`--no-run --message-format=json`, and target directory
`C:/Users/lyh/AppData/Local/Temp/devsweep-settings-core-3034c70d`. The parent
confirmed that the newly created directory has no `LinkType`.

Cargo exited 0. The compiler-artifact executable path was
`<isolated-target>/debug/deps/devsweep_core-2ca6c9bc5b0fca68.exe`, but the
post-build existence check returned `existsBefore: false`. The parent did not
run `--list` or any test body. The record is `BLOCKED`; the parent also saved
the compiler-artifact JSONL and build log.

The missing executable also occurs with an independent, ordinary output
directory. The shared target symbolic link is therefore insufficient to
explain the failure. The cause remains undetermined. No additional diagnostic
command or configuration change was performed for this report update.

## Security event-log follow-up

The parent requested a bounded read-only check for 2026-09-27 21:25 through
22:20 in America/Chicago (CDT, UTC-05:00). The `Get-WinEvent` query used
2026-09-28 02:25:00 through 03:20:00 UTC. This security-log inspection had
not been included in the earlier Application 1000/1001 check.

The requested matching keys were the two exact executable names
`devsweep_core-0942b4e2b82cca0a.exe` and
`devsweep_core-2ca6c9bc5b0fca68.exe`, plus the isolated target directory
`C:/Users/lyh/AppData/Local/Temp/devsweep-settings-core-3034c70d`.
Event XML was checked for these identifiers only after the time filter.
No matching event content was returned.

| Existing local log | Access and result |
| --- | --- |
| `Microsoft-Windows-Windows Defender/Operational` | Readable and enabled; 31 retained records in log metadata; 0 events in the requested interval, therefore 0 identifier matches. |
| `Microsoft-Windows-CodeIntegrity/Operational` | Readable and enabled; 1,217 retained records in log metadata; 0 events in the requested interval, therefore 0 identifier matches. |
| `Microsoft-Windows-AppLocker/EXE and DLL` | Readable and enabled; 0 retained records; 0 events and matches in the requested interval. |
| `Microsoft-Windows-Sysmon/Operational` | Unavailable: `NoMatchingLogsFound`; no matching local log exists. |
| `Security` | Access unavailable: `LogInfoUnavailable`, with `Attempted to perform an unauthorized operation`. The shell could not read log metadata. The planned query for IDs 4656/4660/4663 did not run. |

The completed shell query exited 0; it captured the unavailable-log and
access-denied results as diagnostic records. No available event establishes
which process removed or failed to retain the core executable. The cause
remains undetermined. An empty interval does not prove that no security
product acted, and the unreadable Security log supplies no evidence either
way.

Only this report was appended. No build, test executable, security scan,
remediation, file restoration, quarantine operation, exclusion, elevation,
or security-setting change was performed.
