# Creation Handoff

- skill name: `devsweep-inspect`
- version: 2.0.0
- job: Inspect this machine with the globally installed DevSweep CLI,
  recommend cleanup, and execute only after a displayed list is confirmed.
- path: `skills/devsweep-inspect/`
- publication: not requested; in-repo Production package only

## Reference skills studied

- `avdlee/xcode-disk-cleanup-agent-skill@xcode-disk-cleanup` (212 skills.sh
  installs on 2026-09-01, 51 GitHub stars, MIT): audit-first proposal table.
  Mapped to `references/recommendation-report.md`. Rejected same-turn apply.
- `az9713/claude-skill-disk-cleanup@windows-disk-cleanup` (0 stars, MIT):
  Discover then ask. Mapped to Compact Workflow step 6 and
  `references/confirmed-clean.md`. Rejected `Remove-Item`.
- `thearmagan/skills@dry-run-first` (2 installs, 2 stars, NOASSERTION):
  preview before mutate. Mapped to `clean preview` then digest.
- `affaan-m/ECC@config-gc` (SkillsMP 2026-09-01; parent-repo stars are not
  skill quality): numbered table then human confirmation. Mapped to
  list-and-wait. Rejected per-item `[y/n]` and config-directory `du`/`mv`.
- `vinta/awesome-python@preview-verdicts` (SkillsMP 2026-09-01; parent-repo
  stars are not skill quality): wait for explicit go. Mapped to the
  confirmation turn boundary. Rejected HTML review pages.

## Absorbed and rejected

- keep: inspect permission is not execute permission; evidence table; preview
  before side effects.
- adapt: DevSweep five-mode CLI; `CONTEXT.md` terms; confirmation list instead
  of recommend-only stop.
- reject: elevation, Docker, Cargo home cleanup, shell deletion, software
  uninstall, same-turn execute, per-item y/n, Qiaomu public-brand copyright,
  `cargo run` of this repository.
- invent: `scripts/resolve_devsweep.py`, `scripts/list_selectable.py`,
  `scripts/plan_selected.py`; exclude current repo from default selection.

## Advantages

- Design advantage: PATH `devsweep.exe` only; refuse this checkout's `target/`
  binary.
- Design advantage: cleanup list is a required turn boundary.
- Validated advantage: trigger eval passed 16/16 (`reports/trigger-eval.json`).
- Validated advantage: output fixture eval passed 4/4
  (`reports/output-eval.json`).
- Hypothesis: operator install proof remains missing evidence.

## Verification and limits

- `validate_skill.py skills/devsweep-inspect` must exit 0.
- `SKILL.md` stays under the 14000-byte Production budget.
- Install proof, provider-backed comparison, and human review remain
  missing evidence.
- `optimize run` and `software uninstall` stay out of this skill.
