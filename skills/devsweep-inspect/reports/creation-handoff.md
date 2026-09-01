# Creation Handoff

- skill name: `devsweep-inspect`
- version: 1.0.0
- job: Inspect this machine with the DevSweep CLI and return cleanup and
  optimization advice without side effects.
- path: `skills/devsweep-inspect/`
- publication: not requested; in-repo Production package only

## Reference skills studied

- `avdlee/xcode-disk-cleanup-agent-skill@xcode-disk-cleanup` (212 skills.sh
  installs on 2026-09-01, 51 GitHub stars, MIT): audit-first proposal table.
  Mapped to `references/recommendation-report.md` and
  `references/safety-contract.md`. Rejected same-turn apply after category
  approval.
- `orzcls/win-disk-cleaner@win-disk-cleaner` (109 installs, 4 stars, MIT):
  Chinese and English disk-space triggers. Mapped to `SKILL.md` description.
  Rejected admin PowerShell cleanup.
- `az9713/claude-skill-disk-cleanup@windows-disk-cleanup` (0 stars, MIT):
  Discover then ask. Mapped to Compact Workflow step 6. Rejected
  `Remove-Item` execute scripts.
- `heyzgj/storage-cleanup-skill@storage-cleanup` (6 installs, license
  unrecorded): categorized advice table. Mapped to advice values
  recommend / inspect-only / exclude. Rejected `du` / `rm -rf`.
- `thearmagan/skills@dry-run-first` (2 installs, 2 stars, NOASSERTION):
  preview before mutate. Mapped to `clean preview` in the playbook. Rejected
  PowerShell `-WhatIf` as the DevSweep substitute.

## Absorbed and rejected

- keep: inspect permission is not execute permission; evidence table; preview
  before side effects.
- adapt: DevSweep five-mode CLI as the only scanner; `CONTEXT.md` terms.
- reject: elevation, Docker, Cargo home cleanup, shell deletion, software
  uninstall, Qiaomu public-brand copyright.
- invent: recommend-only Production package bound to DevSweep inspect argv.

## Advantages

- Design advantage: DevSweep scan → untrusted Cleanup Plan → digest chain
  instead of composed cleanup commands.
- Design advantage: recommendation output must use `CONTEXT.md` terms.
- Validated advantage: trigger eval passed 13/13 (`reports/trigger-eval.json`).
- Validated advantage: output fixture eval passed 2/2
  (`reports/output-eval.json`).
- Hypothesis: Grok description auto-discovery is weaker until `AGENTS.md`
  wiring. Install proof remains missing evidence.

## Verification and limits

- `validate_skill.py skills/devsweep-inspect` exits 0.
- `SKILL.md` is 2720 bytes, under the 14000-byte Production budget.
- Install proof, provider-backed comparison, and human review remain
  missing evidence.
- This skill does not run `clean execute`, `optimize run`, or
  `software uninstall`.
