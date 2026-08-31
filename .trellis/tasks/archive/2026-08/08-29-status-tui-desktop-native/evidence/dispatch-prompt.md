Active task: .trellis/tasks/08-29-status-tui-desktop-native

You are already the trellis-implement sub-agent. Implement directly. Do NOT spawn trellis-implement or trellis-check. Do NOT git commit, push, merge, or amend.

<!-- trellis-hook-injected -->
Hook injection did not fire; load context from the Active task path.

## Repo

D:\Documents\Code\Rust\Exp\devsweep — Windows PowerShell, branch `dev`.
Collector child is archived at `.trellis/tasks/archive/2026-08/08-29-status-collector-cli/`. Do not reopen it. Do not add collectors or persistence. Do not edit `cli.rs`. Do not edit shell-owned desktop spec.

## Load

implement.jsonl + listed files, prd.md, design.md, implement.md, `.trellis/spec/frontend/index.md` and `.trellis/spec/desktop-frontend/index.md` checklists. Use Optimize/Software presentation children as structural precedent (`tui/modes/optimize`, `desktop/src/modes/optimize`) without copying their domains.

Parent Status R4 resource gates (must meet on release-build host):
- snapshot p95 <= 2 s
- peak private bytes <= idle+64 MiB, threads <= idle+4
- 60 s live: median CPU <= 5% and p95 <= 15% of one logical core; same private/thread caps
- within 5 s after exit: 25-sample post-exit trace; both thresholds in every final-five sample; fail on a missing sample

## Contracts

R1–R4 / AC1–AC4. Snapshot first; live only by explicit action. At most 60 in-memory points; drop on mode exit. Charts gap missing samples, never interpolate as real zeros. Text alternatives for charts. Frozen Status V1 only. Live mutually exclusive with Scan/Analyze/Software/Optimize via shell coordinator. Pause/leave/close cancels and joins. No GPU/thermal/fan zero cards. Process privacy: no path/cmdline/user.

## Forbidden

`.trellis/.gitignore`, `README.md`, `justfile`, other task dirs except this one. No `git add -f .trellis/`. No push/amend/sign. No unapproved deps. No UAC. No background live.

## Required

1. Implement design file-level change list including `docs/validation/status-native.md`.
2. Focused gates with logs under this task `evidence/` (command + real exit):
   - `rtk cargo test -p devsweep-cli status`
   - `rtk cargo test -p devsweep-cli tui::modes::status` if that module exists
   - `rtk just desktop-web-check` (and underlying npm)
   - `rtk just desktop-test`
   - `rtk just desktop-build`
   - `rtk git diff --check`
3. Native GUI bar (completion-required; do not mark UNVERIFIED): EN/ZH, keyboard/SR, reduced motion/high contrast, 390/800/1024/1440, device-scale 100/125/150/200 (do not change OS global scale), explicit live start, interval change, churn, leave/close, restart. Screenshots + AX/CDP as Software/Optimize presentation did.
4. Resource gate on release binary with 25-sample post-exit. Save raw samples.
5. `evidence/verification-report.md`, implement.jsonl notes, `evidence/dispatch-prompt.md`.

Report Implementation Complete. Do not commit.
