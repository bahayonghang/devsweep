# Final integration gates: approved step 9

Date: 2026-11-16. Workspace: dev, HEAD
`faf9dc6d1e2f582ffa3183350d083e1fcf906f60`. These commands cover the combined
uncommitted worktree, including prior Clean and Host 3–8 changes. This is not a
claim that all Host Actions or native UI acceptance are complete. Step 10 stays
unopened, with no commit, push or archive.

## Commands and exact results

Canonical Cargo operations ran serially after product source was frozen. No
other Cargo operation overlapped them.

| Command | Actual result |
| --- | --- |
| `$env:RUST_TEST_THREADS='4'; just ci` | **Pass, exit 0.** Root format, all-target type-check, tests and Clippy with `-D warnings`: **925 passed, 0 failed, 3 ignored**, across 14 test binaries. |
| `$env:RUST_TEST_THREADS='1'; just gpui` | **Pass, exit 0.** Format, all-target Clippy with `-D warnings`, **240 passed, 0 failed, 0 ignored** in 29.68 s, and locked release build in 42.23 s. No window launched. |
| Python full suite, first run | **Fail, exit 1.** `python -m unittest discover -s tools/tests`: **23 tests, 2 failures, 1 error**, 4.488 s. |
| Python focused follow-up | **Pass, exit 0.** `python -m unittest discover -s tools/tests -p test_skill_distribution.py -k test_resync_restores_match_and_leaves_source_hash_unchanged`: **1 test, 0 failures/errors**, 0.091 s. |
| Python full suite, second run | **Fail, exit 1.** Same complete command, **23 tests, 2 failures, 1 error**, 3.891 s. The extra error recurred; no further retries. |
| `git diff --check` | **Pass, exit 0.** |
| `python .trellis/scripts/task.py validate .trellis/tasks/11-12-gpui-host-actions` | **Pass, exit 0.** 14 implement entries, 11 check entries; one context-size warning below. |

Neither Rust canonical gate required a retry, skipped test, relaxed timeout or
weakened assertion. All five provider_authority cases passed in the first CI
run. Both all-target Clippy builds also establish the type-check result.

### Root test binaries

| Test binary | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| CLI library | 215 | 0 | 0 |
| CLI main | 0 | 0 | 0 |
| Process fixture | 0 | 0 | 0 |
| c01_shipped_parity | 8 | 0 | 0 |
| c05_software_maintenance | 2 | 0 | 0 |
| c07_analyze_parity | 2 | 0 | 0 |
| c15_ai_cleanup | 12 | 0 | 0 |
| cli_contract | 17 | 0 | 0 |
| five_mode_contract | 8 | 0 | 0 |
| keep_awake_contract | 2 | 0 | 0 |
| provider_authority | 5 | 0 | 0 |
| Core library | 652 | 0 | 3 |
| Windows action helper | 0 | 0 | 0 |
| public_api | 2 | 0 | 0 |
| **Total** | **925** | **0** | **3** |

The existing ignored cases remain:

- optimize::tests::windows::native_settings_shell_execute_probe: opt-in native
  Settings ShellExecuteExW probe.
- software::msix::phase_benchmark::profile_msix_inventory_phases: read-only
  host benchmark.
- software::msix::phase_benchmark::profile_msix_metadata_calls: read-only
  metadata diagnostic.

Ignored tests are not passed native acceptance.

## Python suite remains non-green

Both complete runs retained the two known failures, subtests of
test_gitignore.TestGitignore.test_generated_and_local_paths_are_ignored:

1. target/debug/devsweep.exe: Git exit 128, pathspec is beyond a symbolic link.
2. .trellis/hooks.local.json: Git exit 1, the path is not ignored.

Both complete runs also reported an additional error in
test_skill_distribution.TestSkillDistribution.test_resync_restores_match_and_leaves_source_hash_unchanged.
Its first `_install()` at test_skill_distribution.py:105 reaches
skill_distribution.py:218, `staging.replace(dest)`, and Python raises
PermissionError / WinError 5 (Access is denied). The affected paths are private
temporary `.claude/skills/.demo-skill.installing` -> `demo-skill` directories:
first run devsweep-skill-dist-r2bpt3d2; second run devsweep-skill-dist-apnyg50w.

The focused run used another isolated fixture and passed, but the subsequent
complete rerun reproduced the error. This does not establish an ACL, antivirus
or ordering root cause. Both tools files have no worktree diff; the error is
outside the approved process-action implementation. Tools, ACLs and ignore
rules were left unchanged. No test was excluded from the full runs. The two
failures plus one error are the final complete-suite result, not a green suite
and not merely the previously known two failures.

Other skill-check-failed messages are expected negative-fixture diagnostics;
they are distinct from the actual PermissionError and final unittest counts.

## Logs and diagnostics

RTK emitted full canonical test-output logs at:

- `C:\Users\lyh\AppData\Local\rtk\tee\1794808396_just_ci.log`
- `C:\Users\lyh\AppData\Local\rtk\tee\1794808600_just_gpui.log`

Exact counts above were parsed from their test-result lines. Compiler/recipe
progress and successful release completion were also returned by the tool.
Python and context-validation outputs were returned untruncated; no separate
RTK log path was emitted for them. Their complete-suite counts and actual error
site are preserved here.

Context validation warns that quality-guidelines.md is 76,558 bytes, exceeding
the 32,768-byte injection limit. Validation still passes. Applicable source
sections and the process-specific guide were read directly during review.

Nonfatal mbx messages included a cache-store Access-is-denied result and absolute
fixture/dependency paths without stable cache mappings. Rust gates completed
successfully; no cache configuration was changed. These messages do not prove
the cause of the separate Python temporary-directory error.

## Release artifact

The locked GPUI recipe built this file with `RUSTFLAGS=-C link-arg=/Brepro`:

```text
D:\Documents\Code\Rust\Exp\devsweep\desktop-gpui\dist\goal-gpui-release\release\devsweep-gpui-spike.exe
size:   21,054,464 bytes
sha256: cc6922d83d7954590380509ec33f36b1ef81c846fd494375e8e32edc3fb5ae32
```

The hash was read after the completed build. The artifact was not launched or
installed; generated dist content remains uncommitted.

## Evidence boundaries

The step-9 findings, self-fixes and first focused assertion failure are recorded
in [process-step9-check.md](process-step9-check.md). Both core and CLI existing
owned-process tests create and terminate their own ping child; the CLI case
also proves wrong-FILETIME refusal on the actual termination handle and uses a
test-only memory journal. Other process-action controllers use fake probes and
killers, and file journals are isolated. Native helper/owned-child coverage is
not native GPUI keyboard, accessibility, rendering or screenshot acceptance.

No existing user process was ended. No host install/uninstall/update, startup
change or real power request ran. No Computer Use or desktop window was used.
Native screenshots and later Software actions remain unverified/unopened.
