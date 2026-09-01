# Production inspect-advise skill package

## Goal

Author the Production skill package at `skills/devsweep-inspect/` so an agent
can inspect this machine with the DevSweep CLI and return cleanup and
optimization advice without side effects.

## Confirmed facts

- Parent: `.trellis/tasks/09-01-inspect-advise-skill/`.
- Qiaomu Production requires `SKILL.md`, `README.md`, `agents/interface.yaml`,
  `manifest.json`, `evals/trigger_cases.json`, and evidence reports
  (`reports/skill-ir.json`, `reports/trigger-eval.json`,
  `reports/prior-art-research.md`, `reports/creation-handoff.md`).
- `validate_skill.py` limits Production `SKILL.md` to 14_000 bytes and allows
  one root `SKILL.md` inside the package.
- Canonical CLI is `docs/reference/cli.md`. Human guides:
  `docs/guide/clean.md`, `docs/guide/optimize.md`,
  `docs/guide/safety-model.md`.
- Prior-art synthesis lives in the parent research file.

## Requirements

- R1: Create `skills/devsweep-inspect/` with skill name `devsweep-inspect`,
  owner DevSweep/`bahayonghang`, MIT via the repository license, maturity
  `production`.
- R2: Root `SKILL.md` must route on inspect/advice triggers, keep the workflow
  short, and point judgment to `references/`. Description must include English
  and Chinese natural triggers and explicit exclusions for execute, uninstall,
  TUI work, and ad-hoc deletion.
- R3: References must document the inspect CLI playbook, the safety contract,
  and the recommendation-report shape. Do not duplicate the full CLI reference.
- R4: Default workflow may run only read-only inspect commands:
  `clean scan`, `clean rules list`, `clean protect list`, `clean preview`,
  `analyze scan`, `optimize list`, `status snapshot`, `history list`.
  `clean plan` is allowed only when the user asked to save a plan file.
- R5: The workflow must forbid `clean execute`, `optimize run`,
  `software uninstall`, `clean protect add|remove`, `rm`, `Remove-Item`,
  and shell-composed cleanup.
- R6: Recommendation output must use Cleanup Target, Scan Report, Cleanup
  Plan, Estimated Recoverable, and Inspect Only. Incomplete sizes stay
  partial-lower-bound or unknown.
- R7: `evals/trigger_cases.json` must cover should-trigger, should-not-trigger,
  and near-neighbor cases, including execute and uninstall requests.
- R8: Add a small output eval that fails if a recommendation includes execute
  commands or shell deletion.
- R9: Copy parent prior-art into `reports/prior-art-research.md`, export Skill
  IR, run trigger eval, and write `reports/creation-handoff.md`.
- R10: `python C:\Users\lyh\.grok\skills\qiaomu-meta\scripts\validate_skill.py skills/devsweep-inspect`
  and trigger eval must pass.

## Acceptance Criteria

- [x] AC1 (R1, R2, R3): Package layout matches Production qiaomu files and
      `SKILL.md` stays a router plus minimal workflow.
- [x] AC2 (R4, R5, R6): The written workflow inspects with DevSweep CLI, uses
      domain terms, and contains an explicit execute prohibition.
- [x] AC3 (R7, R8, R9, R10): `validate_skill.py` exits 0; trigger eval meets
      the package threshold; output eval covers the execute/shell-deletion
      prohibition; evidence reports exist.

## Out of scope

- Discovery edits in `AGENTS.md` or `code_map.md` (child
  `09-01-inspect-skill-discovery`).
- Changing DevSweep CLI, core, TUI, or desktop.
- Publishing or installing via `npx skills add`.
- A gated execute path.
