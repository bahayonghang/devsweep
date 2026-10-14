# Functional implementation notes

## Coordination

2026-10-13: Coordinator released product edits after original reference A (CLI, native Tauri, embedded frontend, source archive and fixture manifest) was preserved. Core scanner/rules/provider authority and CLI presentation belong to this implementer. The runtime helper implements desktop scope/persistence/review/rescan. Earth implementer delivered the isolated Windows cache proof module; its ownership has returned to the functional implementer. Main owns canonical specs and the new Windows full-pipeline fixture.

Functional tests and metadata are stable; canonical `just ci` now passes. Release CLI B is building, then the runtime helper will capture backend source identity and native Tauri B with its already preserved frontend. Runtime production edits wait for both artifacts and the source snapshot. No user cache has been cleaned; command fixtures only record invocation, trash fixtures only record paths.

## Current evidence and open checks

- Real CLI `provider_authority` integration: three tests passed, exercising nine owner variants, four stale identity/config replacements, seven default config sources and credential-only exclusion through scan, saved plan, preview and simulated native command. See `evidence/provider-authority-validation.md`.
- Preserved `dist/native-test-isolation/core-functional.exe` SHA-256 `4149d414b7fc105a38f248433c862a0de2f2e79539a30fe0b059311394e3d096`: 23 rule tests passed, including positive/retained-secret/tracked-file checks for all 14 generated artifact declarations and nine Windows helper tests.
- The same preserved binary passed 69 of 71 scanner/model tests. Two fixture failures were the relocated process companion not being copied beside the test binary, and the old Windows ACL fixture failing to deny `read_dir`. Companion placement and precise `*S-1-1-0:(RD)` fixture ACL/RAII restoration are being corrected. These are not a canonical gate pass.
- Earlier commands from `target/debug/deps` intermittently ended with `0xc0000005` (access violation), and a serial generated test ended with `0xc000001d` (illegal instruction); binaries subsequently disappeared. Copies in `dist/native-test-isolation` execute successfully. Root cause remains unproven; preserve exact failures and use a controlled workspace-local Cargo target for the final gate, without excluding tests.
- Full generated scan/plan/preview/fake-trash matrix passed for all 14 new outputs. The Windows seven-scenario full pipeline passed for all six families and 22 executable cache children; it exposed and fixed shared inspect-only fingerprints for separate Temp entries. Existing-rule pipelines now also pass for the ten directory rules, nested Python bytecode, Cargo command/fallback and default/configured known global roots. The current rules group passes all 24 tests, including case-insensitive retained names and independent mixed Firefox profile outcomes.
- Controlled `cargo test --locked --target-dir dist -p devsweep-core --lib` initially ran 413 cases without a native crash (407 passed, five failed, one ignored). The five failures were the Windows inspect fingerprint bug, ineffective sizing ACL fixture, inherited Cargo target override, unexplained native DNS operation failure and temporary registry fixture permission. These failed attempts are retained in the evidence; the final results below supersede their gate status.
- Final verification supersedes the earlier open gate: core 417 passed / three ignored diagnostic entries; CLI 184 + 12 + 7 + 3 passed; desktop 86 passed / two existing fixture entrypoints ignored. Canonical `just ci` passed with no filtering. DNS test side effects were isolated rather than skipped, native registry permission was verified outside the sandbox, and language/settings fixtures were isolated without changing product locale behavior. See `evidence/native-functional-gate.md` for failures and exact evidence.

## Implemented safety and platform boundaries

Generated output uses exact marker/dependency ownership, required generated evidence, bounded no-follow content traversal, secret/nested-repository denial and Git ignored/untracked proof. New output is unselected. A live native owner snapshot must be idle. The native owner check currently supports Windows; on other platforms these newly added outputs remain inspect-only with an explicit unknown-activity reason. Existing cross-platform rules are unchanged by that new boundary. Content verification is bounded to two seconds and 50,000 visited/pending entries; scanner cancellation is checked while enumerating and passed to bounded Git probes.

Windows application discovery recognizes supported default per-user layouts only. Temp entries are individual, at least seven days old throughout their trees, SID-owned and exclusively openable. Exact profile/cache membership is separate from dynamic owner eligibility so a busy cache remains an observation without becoming authority. Discovery, preview and live dispatch reuse the same closed policies.

## Additive wire contract

`ScanHealth.coverage` is an optional-on-read vector (empty legacy values serialize absent). Each record has `id`, `scope` (`project` or `provider`), optional `path`, `status`, `target_count`, and `detail`. Status is one of `complete`, `empty`, `absent`, `unsupported`, `skipped`, `failed`, `canceled`, `timeout`, `output_truncated`, `budget_limited`, `protected`, `busy`, or `ownership_unknown`.

Project paths report normalized effective roots; invalid attempts retain a safely resolved absolute lexical path. Legacy missing coverage is not evidence that every provider/root was evaluated. Existing top-level completeness remains readable. Optional provider absence/unsupported status is not a failure; failure to evaluate a requested root/provider makes coverage partial.

The existing v2 target `rule_id`, `path`, and `KnownCacheDir` source/path represent untrusted observed provider scope. Derived live provider bindings remain internal and do not serialize executable authority. Pathless legacy provider observations must be rescanned before preview.

## Official sources verified before enabling rules

- [Go command documentation](https://pkg.go.dev/cmd/go): `clean -modcache` removes the entire module-download cache; `clean -cache` targets build cache separately. A successful cache query alone does not authorize an arbitrary path.
- [uv cache documentation](https://docs.astral.sh/uv/concepts/cache/): use the owner cache command; direct cache modification is unsafe. Configuration and command concurrency require provider ownership.
- [Bun global cache](https://bun.sh/docs/pm/global-cache): cache location can be configured through `BUN_INSTALL_CACHE_DIR` and `bunfig.toml`; home-relative guessing cannot establish the active footprint.
- [Yarn cache clean](https://yarnpkg.com/cli/cache/clean): mirror and project cache are distinct footprints; the current `cacheFolder` observation must not represent an unrelated mirror action.
- [npm cache](https://docs.npmjs.com/cli/v11/commands/npm-cache/) and [pip cache](https://pip.pypa.io/en/stable/cli/pip_cache/): preserve owner commands and explicit irreversible review.

These sources establish documented semantics, not installed-tool support or runtime evidence on this machine.
