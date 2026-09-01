# Dispatch prompt (implementer session)

Active task: `.trellis/tasks/08-29-five-mode-native-integration` on branch `dev`.
This leaf is glue-only. Domain defects return to the owning child. Do not reopen
archived implementation leaves. Do not commit, push, sign, publish, or release.

## Inputs used

- `implement.jsonl`, `prd.md`, `design.md`, `implement.md`
- Root `.trellis/tasks/08-29-mole-inspired-cli-windows-desktop-redesign/design.md`
- `.trellis/spec/backend/index.md`
- `.trellis/spec/desktop-frontend/index.md`
- `.trellis/spec/guides/cross-layer-thinking-guide.md`
- Frozen resource protocol table in this child's `design.md`
- Parent acceptance: Scan/Analyze archived PASS; Software/Optimize/Status overall PASS

## Glue delivered

- Shared CLI/TUI/Tauri registration (`SHIPPED_COMMAND_ROOTS`, `SHIPPED_INVOKE_COMMANDS`, `registry.ts`)
- `crates/devsweep-cli/tests/five_mode_contract.rs`
- `tools/measure-resources.ps1 -Protocol five-mode-v1` (persistent CDP serve, StrictMode-safe sampling)
- Docs: `docs/validation/five-mode-native.md`, `docs/guide/cli-migration.md`, `docs/safety-capability-matrix.md`
- `README.md` new `## Five-mode product` hunk only (user `just tinstall` hunk preserved)
- `desktop/src-tauri/tauri.conf.json` packaging metadata, unsigned current-user NSIS
- Native capture: `evidence/native/run-native-matrix.ps1`, `capture-native-five-mode.mjs`

## Forbidden (honored)

Push, publish, sign, release, credentials, paid services, unapproved deps, user
global settings, `git add -f .trellis/`, expanding the 22-item tree, Docker
cleanup, permanent delete, Cargo home mutation, `justfile` / `.trellis/.gitignore`
edits.

## Outcome

Glue implementation is complete. AC1–AC3 and R1–R3/R5 recorded PASS.
AC4 / R4 recorded FAIL on the frozen five-mode-v1 table (see
`evidence/verification-report.md`). Thresholds were not weakened. Analyze /
Software / Optimize numeric misses returned to those children. No commit.

## Verification artifacts

See `evidence/verification-report.md` and `evidence/logs/`.
