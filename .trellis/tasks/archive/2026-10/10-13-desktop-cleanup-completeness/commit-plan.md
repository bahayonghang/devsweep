# Proposed commit plan

Draft for Phase 3.4 review. Nothing is staged or committed. This inventory is
not a commit request. As of the C4 measurement checkpoint the worktree has
146 `git status --short` lines, so the path lists below are no longer a
complete inventory. Combined GUI acceptance and the formal Software timing
gate are still open, so this file is not refreshed into an approval-ready
grouping.

Refresh this inventory after final acceptance; unresolved gates remain in acceptance-status.md and are not waived by this plan.

## 1. Implement the approved cleanup, Earth and runtime changes

Proposed subject: `feat: improve cleanup coverage, Earth rendering and runtime bounds`

Includes production code, regression tests, packaged assets and notices, measurement tools, bilingual usage guidance, navigation map and prospective coding contracts. Keeping the integrated source together avoids partial commits across shared DTO, provider, rendering and lifecycle boundaries.

Exact paths (143):

- `.trellis/spec/backend/bounded-delivery.md`
- `.trellis/spec/backend/desktop-runtime.md`
- `.trellis/spec/backend/error-handling.md`
- `.trellis/spec/backend/index.md`
- `.trellis/spec/backend/provider-authority.md`
- `.trellis/spec/backend/quality-guidelines.md`
- `.trellis/spec/backend/runtime-publication.md`
- `.trellis/spec/backend/scan-coverage.md`
- `.trellis/spec/backend/scan-preferences.md`
- `.trellis/spec/desktop-frontend/component-guidelines.md`
- `.trellis/spec/desktop-frontend/index.md`
- `.trellis/spec/desktop-frontend/state-management.md`
- `.trellis/spec/desktop-frontend/type-safety.md`
- `.trellis/spec/frontend/hook-guidelines.md`
- `.trellis/spec/frontend/index.md`
- `Cargo.lock`
- `code_map.md`
- `crates/devsweep-cli/src/application/commands/clean.rs`
- `crates/devsweep-cli/src/bin/process_fixture.rs`
- `crates/devsweep-cli/src/tui/render/tests.rs`
- `crates/devsweep-cli/src/tui/runtime/mailbox.rs`
- `crates/devsweep-cli/src/tui/runtime/mod.rs`
- `crates/devsweep-cli/src/tui/runtime/tests.rs`
- `crates/devsweep-cli/src/tui/runtime/workers.rs`
- `crates/devsweep-cli/tests/cli_contract.rs`
- `crates/devsweep-cli/tests/five_mode_contract.rs`
- `crates/devsweep-cli/tests/provider_authority.rs`
- `crates/devsweep-core/src/desktop_preferences/mod.rs`
- `crates/devsweep-core/src/desktop_preferences/store.rs`
- `crates/devsweep-core/src/execution/existing_rule_tests.rs`
- `crates/devsweep-core/src/execution/mod.rs`
- `crates/devsweep-core/src/execution/provider.rs`
- `crates/devsweep-core/src/execution/safety/mod.rs`
- `crates/devsweep-core/src/execution/windows_cache_tests.rs`
- `crates/devsweep-core/src/filesystem/mod.rs`
- `crates/devsweep-core/src/filesystem/sizing.rs`
- `crates/devsweep-core/src/lib.rs`
- `crates/devsweep-core/src/model/scan.rs`
- `crates/devsweep-core/src/optimize/tests.rs`
- `crates/devsweep-core/src/process/executable.rs`
- `crates/devsweep-core/src/process/mod.rs`
- `crates/devsweep-core/src/process/provider_dispatch.rs`
- `crates/devsweep-core/src/rules/configuration.rs`
- `crates/devsweep-core/src/rules/definitions.rs`
- `crates/devsweep-core/src/rules/generated.rs`
- `crates/devsweep-core/src/rules/mod.rs`
- `crates/devsweep-core/src/rules/registry.rs`
- `crates/devsweep-core/src/rules/windows_caches.rs`
- `crates/devsweep-core/src/scan_preferences.rs`
- `crates/devsweep-core/src/scan/global/mod.rs`
- `crates/devsweep-core/src/scan/global/probe.rs`
- `crates/devsweep-core/src/scan/mod.rs`
- `crates/devsweep-core/src/scan/project/mod.rs`
- `crates/devsweep-core/src/software/msix.rs`
- `crates/devsweep-core/src/status/channel.rs`
- `crates/devsweep-core/src/status/mod.rs`
- `crates/devsweep-core/src/status/sampler.rs`
- `desktop/clean-render-benchmark.html`
- `desktop/package.json`
- `desktop/planet-render-benchmark.html`
- `desktop/public/earth/natural-earth.png`
- `desktop/public/earth/still.png`
- `desktop/public/THIRD_PARTY_NOTICES.txt`
- `desktop/scripts/generate-types.mjs`
- `desktop/scripts/prepare-earth-assets.py`
- `desktop/src-tauri/Cargo.toml`
- `desktop/src-tauri/src/async_runtime.rs`
- `desktop/src-tauri/src/commands.rs`
- `desktop/src-tauri/src/lib.rs`
- `desktop/src-tauri/src/scan_preferences.rs`
- `desktop/src-tauri/src/scan.rs`
- `desktop/src-tauri/src/service_boundary.rs`
- `desktop/src-tauri/src/tray.rs`
- `desktop/src-tauri/src/wire_parity.rs`
- `desktop/src/api/bridge.test.ts`
- `desktop/src/api/bridge.ts`
- `desktop/src/api/contract.ts`
- `desktop/src/api/fixture-bridge.ts`
- `desktop/src/api/fixtures/clean/scan-coverage.json`
- `desktop/src/api/fixtures/clean/scan-preferences.json`
- `desktop/src/api/fixtures/contract-variants.json`
- `desktop/src/api/scan-coverage.test.ts`
- `desktop/src/api/types.gen.ts`
- `desktop/src/app-shell/registry.ts`
- `desktop/src/App.test.tsx`
- `desktop/src/App.tsx`
- `desktop/src/components/TargetTable.test.tsx`
- `desktop/src/components/TargetTable.tsx`
- `desktop/src/lifecycle.test.ts`
- `desktop/src/lifecycle.ts`
- `desktop/src/modes/analyze/render-benchmark.tsx`
- `desktop/src/modes/clean/CleanWorkbench.test.tsx`
- `desktop/src/modes/clean/CleanWorkbench.tsx`
- `desktop/src/modes/clean/coverage-results.test.tsx`
- `desktop/src/modes/clean/render-benchmark.tsx`
- `desktop/src/modes/clean/ScanCoverage.tsx`
- `desktop/src/modes/clean/ScanRoots.tsx`
- `desktop/src/modes/clean/styles.css`
- `desktop/src/modes/clean/useScanScope.test.ts`
- `desktop/src/modes/clean/useScanScope.ts`
- `desktop/src/pages/ExecutePage.tsx`
- `desktop/src/pages/ReviewPage.test.tsx`
- `desktop/src/pages/ReviewPage.tsx`
- `desktop/src/pages/ScanPage.tsx`
- `desktop/src/pages/ScanPreviewPage.test.tsx`
- `desktop/src/pages/ScanPreviewPage.tsx`
- `desktop/src/stage/earth-renderer.test.ts`
- `desktop/src/stage/earth-renderer.ts`
- `desktop/src/stage/earth-texture.test.ts`
- `desktop/src/stage/earth-texture.ts`
- `desktop/src/stage/planet-renderer.ts`
- `desktop/src/stage/Planet.test.tsx`
- `desktop/src/stage/Planet.tsx`
- `desktop/src/stage/render-benchmark.tsx`
- `desktop/src/stage/styles.css`
- `desktop/src/state/app-state.test.ts`
- `desktop/src/state/app-state.ts`
- `desktop/src/state/large-results.test.ts`
- `desktop/vite.clean-benchmark.config.ts`
- `desktop/vite.planet-benchmark.config.ts`
- `docs/guide/scan.md`
- `docs/provenance.md`
- `docs/zh/guide/scan.md`
- `README.md`
- `resources/i18n/en.json`
- `resources/i18n/zh-CN.json`
- `tools/analyze-layout-benchmark.mjs`
- `tools/clean-render-benchmark.mjs`
- `tools/inspect-native-readonly.mjs`
- `tools/measure-readonly.py`
- `tools/native-clean-smoke.mjs`
- `tools/native-earth-smoke.mjs`
- `tools/native-operation-smoke.mjs`
- `tools/native-scan-consumer.mjs`
- `tools/native-status-lifecycle.mjs`
- `tools/owned-console-smoke.py`
- `tools/planet-render-benchmark.mjs`
- `tools/provider-golden-parity.py`
- `tools/run-analyze-react-commit.mjs`
- `tools/status-sink-smoke.py`
- `tools/summarize-native-status.py`
- `tools/tests/test_measure_readonly.py`
- `tools/tests/test_native_status_summary.py`

## 2. Record the approved task plans and verification evidence

Proposed subject: `docs: record cleanup and Earth implementation evidence`

Includes the parent and three child task plans, context manifests, research and curated evidence. Raw logs, source snapshots, fixtures, process output and screenshots under ignored dist remain local and are not committed. Task archival and developer journal updates require their separate approved closeout step.

Exact paths (94):

- `.trellis/tasks/10-13-cleanup-discovery-execution/check.jsonl`
- `.trellis/tasks/10-13-cleanup-discovery-execution/coverage-matrix.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/design.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/acceptance-summary.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/desktop-functional.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/functional-review-regressions.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/functional-rule-matrix.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/independent-check.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/native-clean-acceptance.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/native-clean-acl-c2.json`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/native-clean-b3.json`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/native-clean-c1.json`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/native-clean-c2.json`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/native-functional-gate.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/provider-authority-validation.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/windows-cache-focused-tests.txt`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/windows-cache-pipeline.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/evidence/windows-cache-validation.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/implement.jsonl`
- `.trellis/tasks/10-13-cleanup-discovery-execution/implement.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/implementation-notes.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/prd.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/research/windows-cache-boundaries.md`
- `.trellis/tasks/10-13-cleanup-discovery-execution/task.json`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/check.jsonl`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/design.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/baseline-a.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/clean-reference-b.json`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/clean-reference-b.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/cli-console-acceptance.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/frontend-reference-correction.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/frontend-rendering.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/harness-deadlines.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/independent-c2-check.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/independent-runtime-check.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/native-operation-acceptance.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/native-operation-c2.json`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/native-precheck.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/reference-builds.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/repaired-provider-golden-parity.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/runtime-acceptance.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/runtime-resumption.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/scanner-tui-status-implementation.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/software-c3-scheduler.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/software-source-profile.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/software-source-summary.json`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/evidence/status-sink-acceptance.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/implement.jsonl`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/implement.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/implementation-progress.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/prd.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/research/frontend-runtime-plan.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/research/tauri-channel-retention.md`
- `.trellis/tasks/10-13-cli-tauri-runtime-performance/task.json`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/acceptance-status.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/check.jsonl`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/commit-plan.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/design.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/implement.jsonl`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/implement.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/implementation-progress.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/prd.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/backend-cleanup-contracts.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/backend-cli-scan-contracts.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/backend-runtime-contracts.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/cleanup-failures-and-tests.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/cli-tauri-performance.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/current-cleanup-gaps.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/mole-reference-and-assets.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/planet-rendering.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/planning-review.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/planning-validation.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/research/runtime-measurement-protocol.md`
- `.trellis/tasks/10-13-desktop-cleanup-completeness/task.json`
- `.trellis/tasks/10-13-desktop-planet-fidelity/check.jsonl`
- `.trellis/tasks/10-13-desktop-planet-fidelity/design.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/asset-source.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/check-review.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/earth-baseline.png`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/earth-production.png`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/interaction-aligned.json`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/native-earth-acceptance.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/native-earth-c1-failure.json`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/native-earth-c2.json`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/renderer-c2-summary.json`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/renderer-c2.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/renderer-full.json`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/renderer-measurements.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/visual-matrix-summary.json`
- `.trellis/tasks/10-13-desktop-planet-fidelity/evidence/visual-matrix.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/implement.jsonl`
- `.trellis/tasks/10-13-desktop-planet-fidelity/implement.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/prd.md`
- `.trellis/tasks/10-13-desktop-planet-fidelity/task.json`

## Review boundary

This inventory is the pending work of the referenced approved programme. No unrelated user change has been identified in the recorded implementation handoffs. Recheck the final diff and newly created paths before staging. Approval authorizes only the listed local commits; it does not authorize pushing, merging, cleaning user data, installation, archival or journal creation.
