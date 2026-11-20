# Final integration gates: approved steps 3–8

Date: 2026-11-15, America/New_York. Workspace: `dev`, base HEAD
`faf9dc6d1e2f582ffa3183350d083e1fcf906f60`. These checks cover the combined
uncommitted worktree, including the existing Clean work and the MSI, Status and
GPUI/Updates slices. No product edits were made during this final gate run.

The approved step-2 decisions and the separate gates before steps 9 and 10 remain
in force. This receipt establishes automated integration results for steps 3–8;
it does not complete all Host Actions or its native screenshot acceptance.

## Commands and results

Commands ran sequentially, with no concurrent Cargo operation:

| Command | Actual result |
| --- | --- |
| `$env:RUST_TEST_THREADS='4'; just ci` | **Pass, exit 0.** Root formatting, workspace/all-target type-check, tests and Clippy with `-D warnings` passed. **915 passed, 0 failed, 3 ignored**, across 14 test binaries, including three zero-test binaries. |
| `$env:RUST_TEST_THREADS='1'; just gpui` | **Pass, exit 0.** GPUI formatting and all-target Clippy with `-D warnings`, **233 passed, 0 failed, 0 ignored**, then locked release build passed. No desktop window was launched. |
| `python -m unittest discover -s tools/tests` | **Fail, exit 1.** Ran **23 tests in 4.978 s**, with **2 failures**, both subtests of the same existing Git-ignore test. Details below. |
| `git diff --check` | **Pass, exit 0.** No whitespace errors. |
| `python .trellis/scripts/task.py validate .trellis/tasks/11-12-gpui-host-actions` | **Pass, exit 0.** `implement.jsonl`: 12 entries; `check.jsonl`: 9 entries. One context-size warning, detailed below. |

No gate failed because of a newly introduced product issue. No test assertion,
ignore rule or timeout was weakened. No focused or full rerun was required.

### Root test counts

| Test binary | Passed | Ignored |
| --- | ---: | ---: |
| CLI library | 213 | 0 |
| CLI main | 0 | 0 |
| Process fixture | 0 | 0 |
| `c01_shipped_parity` | 8 | 0 |
| `c05_software_maintenance` | 2 | 0 |
| `c07_analyze_parity` | 2 | 0 |
| `c15_ai_cleanup` | 12 | 0 |
| `cli_contract` | 17 | 0 |
| `five_mode_contract` | 8 | 0 |
| `keep_awake_contract` | 2 | 0 |
| `provider_authority` | 5 | 0 |
| Core library | 644 | 3 |
| Windows action helper | 0 | 0 |
| `public_api` | 2 | 0 |
| **Total** | **915** | **3** |

The three existing ignored tests were:

- `optimize::tests::windows::native_settings_shell_execute_probe`: opt-in native
  Settings `ShellExecuteExW` probe.
- `software::msix::phase_benchmark::profile_msix_inventory_phases`: read-only
  native host benchmark.
- `software::msix::phase_benchmark::profile_msix_metadata_calls`: read-only native
  metadata-count diagnostic.

They are reported as ignored, not passed native acceptance. All five
`provider_authority` cases passed on this run. The previously observed
`process::tests::endless_stderr_is_capped_with_truncation_mark` failure did not
recur: that test passed in this full root gate.

### Python failures retained

Both failures belong to
`test_gitignore.TestGitignore.test_generated_and_local_paths_are_ignored`:

1. `target/debug/devsweep.exe`: Git returned **128**, with
   `fatal: pathspec 'target/debug/devsweep.exe' is beyond a symbolic link`.
2. `.trellis/hooks.local.json`: Git returned **1**, so the path was not ignored.

These match the two pre-existing failures identified in the dispatch. The
unrelated ignore rules and symlink layout were left unchanged. The Python suite
is **not green**. Other `skill check failed` text in that output comes from
negative skill-distribution fixtures; it did not add failures to the final
unittest result.

### Context and nonfatal diagnostics

Context validation warns that `quality-guidelines.md` is **76,558 bytes**,
exceeding the **32,768-byte** per-file injection limit. Validation still passes;
the warning means an injected excerpt cannot replace reading the applicable
source sections. Public specs/context ownership remained with the main session.

GPUI emitted nonfatal mbx messages about absolute fixture/dependency paths
without stable cache mappings. Compilation, tests and release linking still
passed; no cache or build configuration was changed for this receipt.

## Full logs

RTK's complete canonical-gate logs, used to verify the exact result counts:

- Root CI:
  `C:\Users\lyh\AppData\Local\rtk\tee\1794802761_just_ci.log`
- GPUI:
  `C:\Users\lyh\AppData\Local\rtk\tee\1794802898_just_gpui.log`

The Python result and context validation were returned untruncated by the tool;
no separate RTK log path was emitted for them. Their actual terminal counts,
failure diagnostics and context warning are recorded above.

## Release artifact for optional manual inspection

Built by the canonical locked GPUI recipe with `RUSTFLAGS=-C link-arg=/Brepro`:

```text
D:\Documents\Code\Rust\Exp\devsweep\desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe
size:   20,990,464 bytes
sha256: 72baf530a16b7ac718f4dabf4cc4f7189336a7636a2562fcc1add7771d1c5708
```

The hash was read with `Get-FileHash -Algorithm SHA256`. The artifact was not
launched or installed; generated `dist` content remains outside source changes.

## Evidence boundaries

Executed existing automated suites and their isolated host-action fixtures/test
executors. No manual real software uninstall/update, startup toggle, process
termination or power request was performed. No `just gpui-dev`, native desktop
window or Computer Use was used. Temporary fixture databases and read-only
native probes do not establish real installed-package acceptance.

Effective Windows power behavior, installed WindowsApps resolution/ACL behavior,
third-party MSI custom-action/UI/UAC/reboot behavior, and English/Chinese native
review/result screenshots remain unverified. Passing text projections, source
guards and fake executors does not satisfy that native screenshot clause.

No commit, push, archive or task-status change was made. The main session owns
spec/checklist synchronization and must retain the review stop before step 9.
