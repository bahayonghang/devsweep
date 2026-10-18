# C15 Implementation and verification

Status: implemented on `codex/gpui-desktop-rewrite`, not accepted. No commit. C15-AC1, C15-AC2, C15-AC3, C15-AC4, and C15-AC5 stay checked. AC4 evidence is the 2026-10-17 rerun of `ac4_worktree_protections_refuse_deletion_policy`, exit 0, log `C:\Users\lyh\AppData\Local\Temp\devsweep-c15-ac4.txt`. AC4 is `deletion_policy` before a move. AC2 now has a generic-scan fixture (`generic_clean_scan_does_not_select_protected_ai_paths`); the AI planner tests remain. AC3 now names `ac3_unknown_sessions_active` and `ac3_unregistered_worktree`.

`clean ai` leaves: `scan`, `plan`, `preview`, `execute`, `recovery-list`, `restore-plan`. Scan, plan, preview, recovery-list, and restore-plan do not mutate the home. Execute is the only user-data mutation, and it uses the existing move-to-trash path after a recovery bundle. A clean disposable worktree that passes every Git check still returns `deletion_policy` before the bundle write and before trash. There is no `--execute` flag and no permanent delete.

## Commands and exit codes

Logs: `C:\Users\lyh\AppData\Local\Temp\grok-goal-8f19d0d75913\implementer\`.
Exit file: `c15-checks.txt`.

| Command | Saved log | Exit |
|---|---|---|
| `cargo test --locked -p devsweep-core --target-dir dist/ci-target` | `c15-core-test.txt` | 0 |
| `cargo test --locked -p devsweep-cli --target-dir dist/ci-target` | `c15-cli-test.txt` | 0 |
| `just ci` | `c15-just-ci.txt` | 0 |

The saved logs are the passing reruns. An earlier CLI run exited 101 because `clean ai execute --confirm` was missing from the value-less flag case list. An earlier `just ci` exited 1 on two clippy warnings in the new AI module. Both were fixed and rerun. The checks file records the final exits: `core=0`, `cli=0`, `just=0`.

Core log: 508 passed, 0 failed, 3 ignored. CLI log includes lib 201 passed and `c15_ai_cleanup` 12 passed. `just ci` finished with `ci complete` after format, tests, and `cargo clippy --workspace --locked --all-targets --target-dir dist/ci-target -- -D warnings`.

No `CARGO_TARGET_DIR`, `HOME`, `CARGO_HOME`, or `RUSTUP_HOME` was set on those processes. Tests pass absolute `--home` on temp directories. This machine was not a `clean execute` or `optimize run` target.

## Acceptance evidence

Checked in `prd.md` because the saved logs show the cases:

- C15-AC1: `default_and_custom_roots_do_not_scan_a_profile`, `ac1_locked_and_malformed_are_not_targets`, `ac1_ac2_three_providers_share_class_rules`, and the CLI class scans. All `ok` in `c15-core-test.txt` or `c15-cli-test.txt`.
- C15-AC2: AI planner tests remain (`ac1_ac2_three_providers_share_class_rules`, `newer_mtime_is_not_authority`, `cursor_electron_cache_children_are_not_targets`, plus CLI class scans). The generic scan is `generic_clean_scan_does_not_select_protected_ai_paths`: isolated-home `Sweeper::default().full_scan_report` (`include_global: true`, `include_projects: false`) keeps current version, active session, credentials, memory, settings, skills, MCP, models, and project content off plan target paths while still observing `.gradle/caches`. Log `C:\Users\lyh\AppData\Local\Temp\devsweep-c15-generic.txt` exit 0.
- C15-AC3: `ac3_bundle_failure_leaves_home_unchanged`, `ac3_failed_move_is_not_retried`, `ac3_disappeared_move_is_unknown_and_not_retried`, `ac3_restore_refuses_external_edit`, `ac3_stale_digest_does_not_mutate` remain. `ac3_unknown_sessions_active` and `ac3_unregistered_worktree` exit 0 (`C:\Users\lyh\AppData\Local\Temp\devsweep-c15-ac3.txt`). CLI: execute without confirm exits 2, stale digest exits 3, bundle-write failure exits 6, restore after an external edit exits 3. Home hashes stay unchanged on the refusal paths.
- C15-AC4: `ac4_worktree_protections_refuse_deletion_policy` is `ok`. Dirty, untracked, ignored, unpushed, submodule, and shared-git-dir worktrees are not targets. The clean disposable worktree is selectable and execute returns `deletion_policy` with zero trash calls, an unchanged home hash, and no `.devsweep-recovery` directory. A later `index.lock` makes that worktree `active_owner`.
- C15-AC5: `ac5_session_enumeration_is_bounded` and `ac5_cancel_before_dispatch_does_not_mutate` are `ok`. `cli_preview_json_matches_core_service` matches the CLI preview JSON `data` to core `preview_ai`, including a live `sha256:` digest.

## NOT_RUN

- CLI process exit 130. The binary has no cancel injection. Core cancellation before dispatch is covered by `ac5_cancel_before_dispatch_does_not_mutate`. No CLI spawn was made to produce exit 130.
- GPUI parity. That remains C11/C14.
- `git worktree remove` was not run. Worktree removal stays `deletion_policy`.
- No `clean execute` or `optimize run` against this machine. No test read the real user profile.
