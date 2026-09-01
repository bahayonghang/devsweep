# Agent CLI inspect-and-advise skill

## Goal

Give coding agents one reusable skill that inspects this machine with the
DevSweep CLI and returns evidence-backed cleanup and optimization advice.
The skill must not execute cleanup, uninstall software, or compose shell
deletion commands.

## User value

Agents currently invent `du`, `rm`, or PowerShell cleanup. That path ignores
DevSweep's scan → untrusted plan → preview digest → confirm chain. A dedicated
skill makes inspect-and-advise repeatable and keeps side effects behind the
existing CLI contract.

## Confirmed facts

- The tracked `skills/` directory exists and is empty. `.agents/` is gitignored.
  Grok auto-discovers `.grok/skills/` and `.agents/skills/`, not `skills/`.
  Repo agents always load root `AGENTS.md`.
- Public CLI roots are `clean`, `software`, `optimize`, `analyze`, `status`, and
  `history` (`crates/devsweep-cli/src/application/cli.rs:41-54`;
  `docs/reference/cli.md:28-67`).
- Clean inspect commands are `clean scan`, `clean plan`, `clean preview`,
  `clean rules list`, and `clean protect list`. Execute requires a saved plan,
  a live `sha256:` preview digest, and `--confirm`.
- `analyze scan` cannot create an executable plan. `status snapshot` is
  read-only. `optimize list` is read-only; `optimize run` is a side effect.
- Domain terms live in `CONTEXT.md`: Cleanup Target, Scan Preview, Scan Report,
  Cleanup Plan, Estimated Recoverable, Inspect Only.
- Safety contracts in `AGENTS.md` and `docs/guide/safety-model.md`: dry-run
  default, no permanent delete, Cargo home Inspect Only, no Docker cleanup,
  program and argv kept separate.
- Prior-art shortlist and keep/adapt/reject/invent notes:
  `.trellis/tasks/09-01-inspect-advise-skill/research/prior-art-research.md`.
- This request is a recurring agent job, not a one-off audit. The earlier
  `08-01-d-drive-space-audit` task used DevSweep scan as inspect-only evidence
  and is a usage example, not a skill package.

## Decisions

- Skill name: `devsweep-inspect`. Product-owned. Do not use a `qiaomu-` prefix
  or 向阳乔木 copyright.
- Mode: Production. Agents will reuse it, and execute-adjacent requests are
  easy to mis-route.
- Canonical path: `skills/devsweep-inspect/`.
- CLI source: DevSweep five-mode CLI only. Do not use ad-hoc `du` or
  `Remove-Item` as the scanner or executor.
- Default action: recommend only. The skill may run read-only inspect commands
  and may write a saved observation or untrusted plan file when the user asked
  for a plan document. It must not run `clean execute`, `optimize run`,
  `software uninstall`, or protection mutations.
- Inspect scope: Clean scan/plan/preview, Analyze scan, Optimize list,
  Status snapshot, optional History list. Software inventory may appear as
  supporting context. Software uninstall stays out of this skill.
- Publication to skills.sh is out of this parent.

## Requirements

- R1: Recurring job. The skill receives a natural-language inspect, disk-space,
  cache, cleanup-advice, or optimization-advice request, plus optional roots
  and scope. It returns a recommendation report. It does not handle execute,
  uninstall, issue triage, or TUI/desktop implementation work.
- R2: Canonical Production package at `skills/devsweep-inspect/` with one root
  `SKILL.md`, `README.md`, `agents/interface.yaml`, `manifest.json`, trigger
  evals, and Production evidence reports.
- R3: Agents must call DevSweep inspect commands with program and argv
  separate. They must not compose shell cleanup strings.
- R4: Recommendation output must use `CONTEXT.md` terms and must separate
  verified, partial-lower-bound, and unknown Estimated Recoverable values.
- R5: The skill must refuse side-effect commands and say that execution needs
  an explicit later user request plus the DevSweep confirm chain.
- R6: Repo agents must discover the skill from `AGENTS.md` and `code_map.md`
  without a second `SKILL.md` copy under `.grok/skills/` in this MVP.
- R7: Child `09-01-inspect-skill-package` owns the package. Child
  `09-01-inspect-skill-discovery` owns discovery wiring and starts only after
  the package child is archived.

## Acceptance Criteria

- [x] AC1 (R1, R3, R4, R5): An agent that follows the skill inspects with
      DevSweep CLI, writes a recommendation report in domain terms, and does
      not execute cleanup or uninstall.
- [x] AC2 (R2): `skills/devsweep-inspect/` validates as a Production qiaomu
      package (`validate_skill.py` and `trigger_eval.py` pass).
- [x] AC3 (R6, R7): After both children are archived, `AGENTS.md` and
      `code_map.md` point at the canonical package, and the parent review
      records PASS or FAIL with evidence paths.

## Out of scope

- Running `clean execute`, `optimize run`, or `software uninstall`.
- Permanent delete, Docker cleanup, Cargo home cleanup, elevation, or UAC.
- Inventing cleanup with `rm`, `Remove-Item`, BleachBit, WizTree, or a bundled
  PowerShell cleaner.
- Publishing the skill to GitHub Releases or skills.sh.
- Changing DevSweep scanner, executor, TUI, or desktop product behavior.
- Copying `.grok/skills/devsweep-inspect` in this MVP.
