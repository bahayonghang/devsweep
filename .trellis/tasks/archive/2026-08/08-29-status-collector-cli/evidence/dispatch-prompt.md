# trellis-implement dispatch — 08-29-status-collector-cli

Active task: `.trellis/tasks/08-29-status-collector-cli`

You are already the trellis-implement sub-agent. Implement directly. Do NOT spawn trellis-implement or trellis-check. Do NOT git commit, push, merge, or amend.

<!-- trellis-hook-injected -->
Hook injection did not fire; load context from the Active task path.

## Repo

D:\Documents\Code\Rust\Exp\devsweep — Windows PowerShell, branch `dev`.
HEAD includes Optimize work (do not revert). `crates/devsweep-core/Cargo.toml` already has Optimize `windows-sys` features (`Wdk_System_SystemServices`, `Win32_System_SystemInformation`, `Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging`, `Win32_System_ProcessStatus`, etc.). **Do not remove them.** Status design says keep the original five and add five collector flags; at HEAD some of those five already exist. Add only the missing ones from the design table (`Win32_NetworkManagement_IpHelper`, `Win32_System_Diagnostics_ToolHelp`, `Win32_System_Power`, and any other listed flag still absent). Prove every added flag is used. No new monitoring crate. No WMI/PowerShell/vendor tools.

## Load first

1. This task implement.jsonl + listed files (research: `.trellis/tasks/08-29-mole-inspired-cli-windows-desktop-redesign/research/windows-status-boundary.md`)
2. prd.md, design.md, implement.md
3. `.trellis/spec/backend/index.md` Pre-Development Checklist files
4. Frozen CLI: `status snapshot` and `status live` already parse; fill `commands/status.rs` and `presentation/status.rs`. Minimal `commands/mod.rs` `status.` dispatch wiring is allowed (same pattern as optimize). Do not edit `cli.rs` parser grammar.

## Contracts

R1–R5 and AC1–AC4. Design: one StatusSampler, 500 ms snapshot window, live 2 s default 1–60 s, process refresh 4 s, volumes/battery 30 s, one in flight, skipped ticks, cancel/join. Process DTO: name, PID, CPU, memory, I/O only — no path/cmdline/user/env/handles/history. Max 4096 enumerated, 150 ms detail budget, default 15/max 100 returned, truncation metadata. Availability tags: available/partial/unavailable/permission-denied/unsupported. Never map missing hardware/access to zero. GPU/VRAM/thermal/fan/SMART/physical-disk activity are static unsupported. JSON snapshot + four frozen NDJSON events. Broken pipe: internal terminal, no second write, cancel/join, exit 0. No persistence/service/tray.

Exact change list in design.md. CLI consumes frozen syntax only.

## Forbidden

- `.trellis/.gitignore`, `README.md`, `justfile`, other `.trellis/tasks/*` except this directory
- `git add -f .trellis/`, commit target/dist, push/amend/sign/release
- Unapproved deps, UAC, sensitive process fields, CleanupPlan from status
- Cleanup execute except dry-run rules in AGENTS.md (this child is read-only)

## Required

1. Implement adapters, sampler, CLI streams.
2. Focused gates with `rtk`, save `evidence/` logs with command + real exit:
   - `rtk cargo test -p devsweep-core status::system` (or honest `tests::` filter if tests live in one tests.rs)
   - `rtk cargo test -p devsweep-core status::process`
   - `rtk cargo test -p devsweep-core status::sampler`
   - `rtk cargo test -p devsweep-cli status`
   - `rtk git diff --check`
3. Native: one warm-up plus five snapshots and one 60-second live sample with 200 ms process sampling; record raw CPU-time deltas, private bytes, thread counts, latency, skipped ticks, post-exit quiescence vs parent limits. Also sleep-sized gaps if reproducible, churn, missing battery, permission denial, 1/60 s bounds, broken pipe, coordinator refusal, no background work.
4. `rtk just ci` as listed. This child does not own desktop UI; do not run desktop-build unless you change desktop files (you should not).
5. `evidence/verification-report.md` and implement.jsonl notes. Copy dispatch to `evidence/dispatch-prompt.md`.

Report Implementation Complete with files, commands+exits+logs, UNVERIFIED ledger (completion-required yes/no). Do not commit.
