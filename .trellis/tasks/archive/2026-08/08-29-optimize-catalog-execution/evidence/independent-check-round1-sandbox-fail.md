# Independent check round 1 — FAIL (channel, not product verdict)

Active task: `.trellis/tasks/08-29-optimize-catalog-execution`
Checker: trellis-check via Cursor `verifier` `5fc662d1-b912-47be-8ce7-40e8c6e5b95d`
Date: 2026-08-31

## Failure signature

`checker_workspace_readonly_no_shell`: the verifier sub-agent reported Ask mode / `workspace_readonly` sandbox; every `rtk`/`just`/native recapture spawn failed; `evidence/independent-check-report.md` was not written.

This is round 1 of max 3 for this signature. Round 2 must change method (writable trellis-check with Shell+Write, not `verifier` Ask mode).

## Checker-stated product concerns (unproven until round 2 re-runs)

- Settings CLI log reuses PID 27684 before/after confirmed run.
- Probe PID 67816 is a later mixed probe; unregistered scheme also returned `h_inst_app=42`.
- Static catalogue/ABI/Cargo.toml review found no contract defect, but independent gates were not run.

Do not treat this file as PASS.
