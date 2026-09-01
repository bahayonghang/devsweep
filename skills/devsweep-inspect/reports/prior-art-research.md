# Prior-Art Research

- Researched at: 2026-09-01 (v1 inspect-only) and 2026-09-01 (v2 confirmed-clean redesign)
- Queries (v1): `disk cleanup inspect recommend agent`; `developer cache cleanup skill`; `safe dry-run file deletion agent`; `disk usage analyze recommend`
- Queries (v2): `confirm before cleanup agent skill`; `preview confirm execute cleanup`
- Catalogs: SkillsMP; skills.sh via `npx.cmd` on v1 only. v2 used `--skip-skills-sh` because Python `subprocess` cannot launch `npx` without `.cmd` on this host.
- Rating evidence: unavailable on both catalogs

## Catalog notes

- SkillsMP v2 returned 18 families. Most hits are keyword collisions (`preview`, `confirm`, `cleanup`) from unrelated repos. Treat SkillsMP popularity as `missing evidence` for this job. Parent-repo stars (for example ECC 244512, awesome-python 317137) are not skill-quality scores.
- skills.sh v1 returned relevant cleanup skills. Install counts are ecosystem adoption telemetry, not ratings.
- `python .../research_prior_art.py --strict` remains unavailable on this host without `npx.cmd`.

## Shortlist

| Candidate | Role | skills.sh installs (2026-09-01) | GitHub stars (2026-09-01) | License | Adopt |
|---|---|---:|---:|---|---|
| [avdlee/xcode-disk-cleanup-agent-skill@xcode-disk-cleanup](https://github.com/avdlee/xcode-disk-cleanup-agent-skill) | Trust/specialist: audit-first evidence table | 212 | 51 | MIT | adapt proposal table; reject same-turn apply after category approval |
| [orzcls/win-disk-cleaner@win-disk-cleaner](https://github.com/orzcls/win-disk-cleaner) | Popularity/Windows trigger language | 109 | 4 | MIT | adapt Chinese/English triggers; reject admin PowerShell |
| [az9713/claude-skill-disk-cleanup@windows-disk-cleanup](https://github.com/az9713/claude-skill-disk-cleanup) | Complementary Discover/Execute split | not in top query list; GitHub source inspected | 0 | MIT | adapt Discover-then-ask; reject `Remove-Item` |
| [heyzgj/storage-cleanup-skill@storage-cleanup](https://github.com/heyzgj/storage-cleanup-skill) | Complementary developer-cache taxonomy | 6 | 0 | none recorded | adapt categorized report; reject `du`/`rm -rf` |
| [thearmagan/skills@dry-run-first](https://github.com/thearmagan/skills) | Complementary preview-before-mutate | 2 | 2 | NOASSERTION | keep preview before side effects; reject `-WhatIf` as DevSweep substitute |
| [affaan-m/ECC@config-gc](https://github.com/affaan-m/ECC/tree/main/skills/config-gc) | Complementary human-in-the-loop GC | SkillsMP only; parent-repo stars are not skill quality | parent repo, not skill-specific | unrecorded here | adapt numbered table then wait; reject per-item `[y/n]` and `du`/`mv` scripts |
| [vinta/awesome-python@preview-verdicts](https://github.com/vinta/awesome-python/tree/master/.claude/skills/preview-verdicts) | Complementary wait-for-go after a review artifact | SkillsMP only; parent-repo stars are not skill quality | parent repo, not skill-specific | unrecorded here | adapt wait-for-explicit-go; reject HTML verdict pages |

Inspected v1 sources remain under
`.trellis/tasks/09-01-inspect-advise-skill/research/prior-art-sources/`.
v2 inspected `config-gc` and `preview-verdicts` `SKILL.md` over HTTPS. Do not
execute candidate scripts.

## What we learned from each candidate

- **xcode-disk-cleanup**: Audit is not delete. Frozen ID table. Same-turn
  execute after category approval conflicts with DevSweep digest + `--confirm`.
  v2 still forbids same-turn execute; it allows execute only after a later
  user confirmation of the displayed list.
- **win-disk-cleaner / windows-disk-cleanup / storage-cleanup / dry-run-first**:
  unchanged from v1. Keep triggers, evidence table, preview habit. Reject
  elevation, `Remove-Item`, `rm -rf`.
- **config-gc**: Never delete autonomously. Present a numbered table. Soft
  delete. It forbids bulk "delete all 15?" and requires per-item `[y/n]`.
  DevSweep cleanup is a named-id batch after one list confirmation, not
  unnamed bulk approval, and not per-row y/n.
- **preview-verdicts**: Generate a review artifact, wait for maintainer go,
  then apply. Too heavy (HTML + localStorage) for this CLI skill.

## Synthesis

- `keep`: inspect is not execute; evidence table; preview/digest before
  mutation; Chinese and English inspect triggers; Trash vs reclaim language.
- `adapt`: Discover-then-ask and wait-for-go onto DevSweep `clean plan` /
  `preview` / `execute --confirm`; SAFE/ASK/KEEP onto recommend / inspect-only /
  exclude.
- `reject`: admin scripts; `rm -rf` / `Remove-Item`; Docker/Cargo home cleanup;
  software uninstall; same-turn execute; per-item `[y/n]` for hundreds of
  developer artifacts; HTML verdict UIs; Qiaomu public-brand copyright;
  `cargo run` of this repository as the cleaner.
- `invent`: PATH-only `devsweep.exe` resolver that refuses this checkout's
  `target/` binary; exclude the current repository from default selection;
  list-and-wait confirmation before `clean execute`; Windows argv batching and
  `validate_plan` skip in `scripts/plan_selected.py`.

## Created skill advantages

- Design advantage: global binary + current-repo exclusion so the product
  checkout is not cleaned as a side effect of invoking itself.
- Design advantage: confirmation list is a required turn boundary, not a
  digest-as-grant shortcut.
- Hypothesis: Grok description auto-discovery still depends on `AGENTS.md`
  wiring. Install proof remains missing evidence.

## Missing evidence

- skills.sh install counts are telemetry, not ratings.
- SkillsMP v2 results were mostly off-topic; no fair SkillsMP ranking.
- v2 skills.sh catalog run: missing evidence (`npx` launch).
- No provider-backed comparison against inspected candidates.
- Install proof and human review: missing evidence.
