# Functional review regressions — 2026-10-13

This checkpoint resumes the seven findings in [the independent boundary review](independent-check.md). The production fixes were already present when this session resumed; they were retained and tested. This session strengthened two regression assertions and did not change production semantics. No real cache cleanup or maintenance command was run.

## Findings and exercised boundaries

| Finding | Current resolution and regression evidence |
| --- | --- |
| F1 | Windows discovery excludes unrepresentable cache roots from plan candidates while retaining typed unavailable coverage. The real isolated Windows pipeline includes a regular-file Chrome `Cache`, a healthy sibling profile and a tox-only project; the complete plan validates and selected healthy paths reach recording runners. The no-follow probe test separately covers cache-root reparse and unverifiable results as well as nested children; retained sentinels survive. |
| F2 | Bounded .NET marker discovery returns typed budget/cancellation/read failures. The ordered iterator fixture puts a real marker after 1,000 entries and proves it is budget-limited rather than absent. A real wide project scan preserves a healthy sibling with partial/budget coverage. |
| F3 | Closed Node entry scripts dispatch through a separately resolved native interpreter with bound script and binary identities. Fixed entry, unsupported wrapper and nonnative interpreter unit cases pass. The real CLI fixture detects same-path Node/script replacement before dispatch. Native execution in this checkpoint is Windows only; the Unix fixture branch remains for platform CI. |
| F4 | Discovery and live Python scope validation share the existing five markers, including `tox.ini`. The real tox-only/nested-tox pipeline passes. The strengthened drift test previews first, creates a nearer `tox.ini`, then attempts recorded execution: one failed outcome names the nearer project boundary, with zero command/trash runner calls. A fresh scan and new preview subsequently succeed. |
| F5 | uv, Bun and Go-build retain prior timeout/canceled/truncated/unsupported probe outcomes; only otherwise malformed path output becomes generic failure. All three provider rows are checked for each typed status. |
| F6 | The selected Yarn rc filename and relevant content participate in the shared configuration policy. Delegated entries are explicitly unsupported, including indented/quoted keys, document markers/directives, custom rc selection and case-insensitive environment delegation. Unit/parser and real CLI scan/preview/drift tests pass; unrelated credentials are not included in authority. |
| F7 | Reviewed size replacement reruns the closed generated/Windows eligibility proof and appends the current refusal. The generated case retains busy explanation, then requires a fresh scan after the owner becomes idle without promoting its intent. The added Windows pipeline assertions cover both busy and unknown owner states after a complete size rescan; refusal text remains, the intent stays inspect-only and default selection stays false. |

## Commands and results

All Cargo invocations used `--locked --target-dir dist` on this Windows checkout. Build/test execution was serialized with the coordinator. Test filters can overlap; the row counts are not a total of distinct tests.

| Command (in addition to the common Cargo arguments) | Result |
| --- | --- |
| `cargo test -p devsweep-core --lib rules:: -- --nocapture` | 26 passed; 0 failed; 5.71 s |
| `cargo test -p devsweep-core --lib scan:: -- --nocapture` | 74 passed; 0 failed; 0.99 s |
| `cargo test -p devsweep-core --lib execution:: -- --nocapture` | 92 passed; 0 failed; 18.60 s, before the two test-only improvements below |
| `cargo test -p devsweep-cli --test provider_authority -- --nocapture` | 4 passed; 0 failed; 10.17 s |
| `cargo test -p devsweep-core --lib execution::tests::existing_rule_pipeline:: -- --nocapture` | 4 passed; 0 failed; 3.59 s, including strengthened F4 |
| `cargo test -p devsweep-core --lib execution::tests::windows_cache_pipeline:: -- --nocapture` | 1 passed; 0 failed; 35.60 s, including added F7 assertions and all eight child scenarios |
| `cargo test -p devsweep-core --lib process::provider_dispatch:: -- --nocapture` | 1 passed; 0 failed; 1.32 s |
| `cargo fmt --all -- --check` | Passed |
| Functional-source `git diff --check` | Passed |

The per-test times above are test execution times reported by Cargo, excluding compilation. All owner-command dispatch tests execute the task's native fixture, which records invocation without cleaning; trash tests use recording runners.

## Handoff

Functional source is frozen for refreshed immutable reference B2. The coordinator owns `just ci`, independent check closure, native packaged acceptance, B2 preservation and final programme acceptance. Performance work must wait until B2 validates. Windows pipeline root-reparse evidence uses the injected no-follow probe; no privilege-dependent link creation was represented as a native link test. Unix execution remains unverified on this host.
