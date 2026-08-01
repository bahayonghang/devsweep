# D Drive Scan Findings

Source: `../../08-01-d-drive-space-audit/research/devsweep-full-d-drive-comparison.md`.

## Observed Results

- Full D: project scan: 657 targets, 688.777 GiB logical estimate.
- Complete observations: 643 targets, 264.658 GiB.
- Partial observations: 14 targets, 424.119 GiB lower bound.
- Rust targets: 34 rows, 643.688 GiB displayed; 27 complete (226.137 GiB) and
  seven partial (417.551 GiB).
- Cargo metadata warnings: 200 total, made up of 186 invalid JSON and 14
  nonzero exits.
- `__pycache__`: 497 rows, about 50 MiB.

## Safety Evidence

- `D:\Documents\LYH` exposes Windows reparse tag `0x9000701a` yet received 34
  cleanup candidates. The metadata-only reparse helper did not protect this
  Cloud Files provider path.
- The audit's Rust cleanup run proved that Cargo rejects directories missing
  `CACHEDIR.TAG`, while devsweep did not make a pre-dispatch authorization
  denial durable or visible in CLI output.
- `quanergy_client_rs\\target\\scratch` contains recently edited scripts and
  data files after Cargo returned success for the parent target. It is evidence
  that post-action outcome reporting must not equate exit success with a claim
  that every descendant was removed.

## Scope Consequence

The implementation must prioritize path-level safety and truthful observation
over expanding what devsweep can delete. Capacity inventory and orphan pnpm
discoveries stay outside cleanup-plan and executor authority.
