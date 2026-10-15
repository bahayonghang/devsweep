# A implementation handoff

Date: 2026-10-14. Product writes began only after the parent confirmed the
source/web/benchmark/native baseline was frozen. No commit, archive, journal,
user-cache scan, cleanup, or installer replacement was performed.

## Implemented contract

- `scan::ranking::rank_cleanup_plan_at` remains the canonical policy. Existing
  freshness/completeness checks run first; any remaining selected project target
  is deselected with one `ranking.project_opt_in` evidence entry. This applies
  to every target kind and old, fully measured Rust command-backed outputs.
- Global defaults, rule IDs, uv behavior, provider binding, explicit CLI
  selection, and TUI confirmation behavior are preserved. No registry,
  scanner implementation, execution path, dependency or public API was added.
- ScanReport V1, CleanupPlan V2, ScanHealth, generated TypeScript and decoders
  have no shape changes. Existing saved plans are not re-ranked on load.
- Core and Tauri tests verify empty-root global requests and fail-closed empty
  project requests. The existing separate flags/CLI defaults are preserved.
  D still owns the two independent desktop actions and submitted-request tests.
- The ranking spec now reflects the actual conservative missing-mtime behavior.

## Changed files

- `crates/devsweep-core/src/scan/ranking.rs`: canonical policy and old/project-kind
  regression; existing freshness fixtures now explicitly represent global scope.
- `crates/devsweep-core/src/scan/mod.rs`: global/project gate, mixed-default and
  closed report roundtrip regressions; publication freshness case uses global scope.
- `crates/devsweep-cli/tests/five_mode_contract.rs`: isolated project scan -> saved
  observation -> explicit selected plan -> live dry-run, with sentinel preservation.
- `crates/devsweep-cli/src/tui/app/tests.rs`: new and old project candidates remain
  unselected, then actual Space selection reaches confirmation without rewriting hints.
- `desktop/src-tauri/src/commands.rs`: scope validation test only.
- `.trellis/spec/backend/{quality-guidelines,scan-coverage}.md` and
  `.trellis/spec/frontend/state-management.md`: executable contracts.
- This task's implementation/evidence documents.

The pre-existing `desktop/src-tauri/Cargo.toml` modification was not touched.

## Verification

Commands run from repository root. Unless noted otherwise, Cargo used
`--locked --offline --target-dir dist`; no `CARGO_TARGET_DIR` environment variable
was set, so Cargo scanner fixtures retained their ordinary authority semantics.

| Check | Result |
| --- | --- |
| New `project_targets_require_explicit_selection_for_every_kind` test before policy change | FAIL as expected: old fully measured project targets remained selected. 0 passed, 1 failed, 448 filtered. |
| `cargo test ... -p devsweep-core scan:: --lib` after policy change | 80 PASS; one unchanged-source generated-output fixture failed because the sandbox denied its temporary `.git/config` write. |
| Exact failed `scan::project::tests::reviewed_generated_rescan_rechecks_refusal_without_promoting_intent`, with sandbox escalation | PASS, 1 test. This only initialized a disposable Git fixture. |
| `cargo test ... -p devsweep-core model::scan:: --lib` | PASS, 7 tests; legacy coverage and closed V1/V2 report decoding remain covered. |
| `cargo test ... -p devsweep-cli --test five_mode_contract project_scan_defaults_are_opt_in_and_explicit_cli_selection_still_previews` | PASS, 1 test; no cleanup execution, sentinel retained. |
| `cargo test ... -p devsweep-cli --test provider_authority` | PASS, 5 tests; production discovery/save/preview plus simulated owner-command dispatch, supported providers including uv, stale executable/script/config refusals and sentinels. |
| `cargo test ... -p devsweep-desktop commands::tests::desktop_scan_never_uses_an_implicit_working_directory --lib` | PASS, 1 test. |
| CLI library suite, including modified TUI selection test | 190 PASS; one unchanged-source protection test failed (details below). |
| CLI contract suite | 11 PASS; one unchanged-source Status snapshot assertion failed (details below). |
| Scoped `rustfmt --edition 2024 --check` on all five Rust files | PASS. |
| `git diff --check` | PASS. |
| Integrated `just ci`, native redesigned UI and desktop request/decoder integration | NOT_RUN by A; parent/F owns the combined gate after B/C/D/E. |

The initial default-target attempt failed opening
`target/debug/.cargo-build-lock` because `target` is an existing link into
`D:/mbx-cache`. One escalated red-test invocation completed there before the
parent supplied the reusable `--target-dir dist` strategy. Subsequent commands
used `dist`; the target link and external cache structure were not changed.

## Observed failures for parent/F triage

These are observations in the modified checkout. The failing source files were
not changed by A. They are **not proven pre-existing failures** because no
same-environment baseline reproduction was performed.

1. Command: `cargo test --locked --offline --target-dir dist -p devsweep-cli
   --lib --test cli_contract --test five_mode_contract --test provider_authority`.
   The library run stopped after 190 passed / 1 failed, before the integration
   targets ran. Failure:
   `application::commands::protect::tests::protect_add_remove_round_trip_and_redacts_audit`,
   `crates/devsweep-cli/src/application/commands/protect.rs:265`:
   `ApplicationError { exit: Unavailable, code: "protection_store_unavailable",
   message: "The protection store is unavailable. Original bytes were preserved
   and cleanup cannot expand.", output_already_emitted: false }`.
   Default sandbox; Rust test thread count not overridden. RTK full log:
   `C:/Users/lyh/AppData/Local/Packages/sandbox.{1cf67eb1-5f0a-4764-9cb7-d72f0b5feb3d}/AC/rtk/tee/1791994794_cargo_test.log`.
2. Command: `cargo test --locked --offline --target-dir dist -p devsweep-cli
   --test cli_contract --test five_mode_contract --test provider_authority`.
   The CLI contract run stopped after 11 passed / 1 failed. Failure:
   `json_and_ndjson_are_locale_neutral_and_keep_diagnostics_off_stderr`,
   `crates/devsweep-cli/tests/cli_contract.rs:213`:
   `live snapshot is closed V1: "status snapshot outcome must reflect
   supported-group availability"`.
   Default sandbox; Rust test thread count not overridden. RTK full log:
   `C:/Users/lyh/AppData/Local/Packages/sandbox.{84a67e42-192e-423f-9550-35a93eaec5ac}/AC/rtk/tee/1791994814_cargo_test.log`.

Independent review and the combined quality gate remain with parent/F. B/C may
consume the finalized scope/selection contract without waiting for a new DTO.
