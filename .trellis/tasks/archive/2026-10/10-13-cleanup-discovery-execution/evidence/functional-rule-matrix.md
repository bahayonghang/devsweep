# Functional rule validation matrix

This is evidence against the approved coverage matrix, not another product catalogue. `rules/definitions.rs` remains authoritative. Status below refers to focused functional proof; final integrated gates and runtime acceptance remain separate.

| Rule IDs / family | Production-chain evidence | Result and limits |
| --- | --- | --- |
| `rust.target` metadata command and local metadata-failure fallback | `existing_rule_pipeline::rust_metadata_and_local_fallback_roundtrip_without_running_cleanup` | Actual Cargo metadata or malformed fixture manifest; real conversion/validation/preview; recording command/trash only; source and output sentinels retained. Inherited CARGO_TARGET_DIR is removed only in a separate default-target fixture child. |
| `node.node_modules`, `node.next_cache`, `node.turbo`, `node.parcel_cache` | `existing_rule_pipeline::existing_project_directories_and_nested_python_roundtrip` | All four real scanner → persisted v2 → registry → live preview → recorded trash cases pass; tampered path refused. |
| `python.venv_dot`, `python.venv`, `python.pytest_cache`, `python.mypy_cache`, `python.ruff_cache`, `python.tox`, `python.__pycache__` | Same existing-project pipeline plus nearest-marker negative fixture | All seven nested paths pass the real chain; no arbitrary descendant name grants authority. New nearer markers cause refusal. |
| `npm.cache.clean`, `pnpm.store.prune`, `pip.cache.purge`, `yarn.cache.clean.classic`, `yarn.cache.clean.modern`, `go.mod_cache.clean`, `go.build_cache.clean`, `uv.cache.clean`, `bun.cache.clean` | CLI `provider_authority` three native integration tests | Nine positive command variants plus same-path tool/config drift and credentials exclusion pass. See provider evidence for exact finite version/layout limits. No package-manager cleanup ran. |
| `gradle.caches`, `gradle.wrapper_dists`, `nuget.packages`, `ivy.cache` | `existing_rule_pipeline::known_global_roots_and_overrides_roundtrip_without_enabling_retained_roots` | Default roots and explicit GRADLE_USER_HOME/NUGET_PACKAGES overrides pass production scanner/registry/preview/recording-trash. Unicode and spaces included. |
| `go.mod_cache`, `maven.repository`, `jetbrains.caches`, `huggingface.hub`, `cargo.home.inspect` | Same known-root pipeline, including GOMODCACHE override | Observations validate, but selecting inspect-only rows for executable preview is refused. Cargo credentials, model/vendor/root sentinels remain. |
| `node.nuxt`, `node.nuxt_output`, `node.svelte_kit`, `node.angular_cache`, `node.astro`, `node.dist`, `node.build`, `node.coverage` | `generated_rules_roundtrip_real_scan_plan_preview_and_fake_trash`; all-rule proof fixture | Every row passes marker/content/Git ignored-untracked proof and real execution pipeline with idle-owner seam; same-path secret introduction refuses live execution. |
| `maven.target`, `gradle.build`, `gradle.project_cache`, `dotnet.bin`, `dotnet.obj`, `python.nox` | Same generated matrix | All six rows pass. Known source directory names alone remain insufficient; source tracking, nested repository, secrets and missing generated evidence refuse authority. |
| `windows.chrome.cache`, `windows.edge.cache`, `windows.firefox.cache`, `windows.vscode.cache`, `windows.cursor.cache`, `windows.user_temp.stale` | `windows_cache_pipeline::real_scan_saved_plan_preview_and_recorded_execution` and nine native helper tests | Six families, 22 positive exact children; seven isolated scenarios include busy/unknown, owner/profile/secret drift and tampering. Retained profile, offline, editor/agent state and secrets checked. See Windows pipeline evidence. |

New generated outputs and Windows children are unselected, medium risk and Recycle-Bin-only. Their sizes are bounded observations, not a promise of freed disk capacity. Native activity proof for newly added generated output currently exists on Windows; other platforms return inspect-only unknown activity. Windows application providers declare unsupported outside Windows. Existing cross-platform rules retain their previous scope/action policy.

Focused commands and outputs:

- `cargo test --locked --target-dir dist -p devsweep-core --lib existing_rule_pipeline -- --nocapture`: three passed, 0.94 s; `dist/native-test-isolation/existing-pipeline-current.txt`.
- `cargo test --locked --target-dir dist -p devsweep-core --lib rules:: -- --nocapture`: 24 passed, 4.46 s; `dist/native-test-isolation/rules-current.txt`.
- Windows full pipeline: one parent test containing seven child scenarios passed, 18.58 s; `dist/native-test-isolation/windows-pipeline-current.txt`.
- CLI provider matrix: three passed, 8.31 s; `dist/native-test-isolation/provider-authority-current.txt`.

Canonical `just ci` subsequently passed in the native permission context; see `native-functional-gate.md`. Remaining acceptance work is platform CI outside Windows, integrated packaged parity and runtime measurements. The idle-owner seam only replaces process activity observation; it does not bypass path, SID/lock, metadata, plan, provider-binding or live safety checks.
