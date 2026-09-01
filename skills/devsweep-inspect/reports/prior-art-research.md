# Prior-Art Research

- Researched at: 2026-09-01
- Queries:
  - `disk cleanup inspect recommend agent`
  - `developer cache cleanup skill`
  - `safe dry-run file deletion agent`
  - `disk usage analyze recommend`
  - extra skills.sh query: `developer cache cleanup`
  - extra skills.sh query: `safe dry-run cleanup`
- Catalogs: SkillsMP (unified runner); skills.sh via `npx.cmd` (Python `subprocess` cannot launch `npx` on this host without `.cmd`)
- Rating evidence: unavailable on both catalogs

## Catalog notes

- SkillsMP returned 26 families. Most hits are keyword collisions from large
  repositories (`openclaw/openclaw` healthcheck/CI, `affaan-m/ECC` developer
  skills). They are not disk-inspect workflows. Treat SkillsMP as
  `missing evidence` for this job, not as popularity proof.
- skills.sh returned relevant cleanup skills. Install counts are ecosystem
  adoption telemetry, not ratings or correctness.
- `python .../research_prior_art.py --strict` failed because `npx` is not an
  `.exe`. Recovery used `cmd /c npx --yes skills find ...`.

## Shortlist

| Candidate | Role | skills.sh installs (2026-09-01) | GitHub stars (2026-09-01) | License | Adopt |
|---|---|---:|---:|---|---|
| [avdlee/xcode-disk-cleanup-agent-skill@xcode-disk-cleanup](https://github.com/avdlee/xcode-disk-cleanup-agent-skill) | Trust/specialist: audit-first, evidence table, explicit IDs | 212 | 51 | MIT | adapt safety and proposal shape; reject Xcode/macOS apply path |
| [orzcls/win-disk-cleaner@win-disk-cleaner](https://github.com/orzcls/win-disk-cleaner) | Popularity/Windows trigger language | 109 | 4 | MIT | adapt Chinese/English triggers; reject admin PowerShell executor |
| [az9713/claude-skill-disk-cleanup@windows-disk-cleanup](https://github.com/az9713/claude-skill-disk-cleanup) | Complementary Windows discover/execute split | not in top query list; GitHub source inspected | 0 | MIT | adapt Discover-then-ask; reject bundled `Remove-Item` execute |
| [heyzgj/storage-cleanup-skill@storage-cleanup](https://github.com/heyzgj/storage-cleanup-skill) | Complementary developer-cache taxonomy | 6 | 0 | none recorded | adapt categorized SAFE/ASK/KEEP report; reject `du`/`rm -rf` |
| [thearmagan/skills@dry-run-first](https://github.com/thearmagan/skills) | Complementary preview-before-mutate habit | 2 | 2 | NOASSERTION | keep preview-before-side-effect rule; reject generic `-WhatIf` as DevSweep substitute |

Inspected sources are stored under
`.trellis/tasks/09-01-inspect-advise-skill/research/prior-art-sources/`.
Do not execute candidate scripts.

## What we learned from each candidate

- **xcode-disk-cleanup**: Treat a cleanup request as audit permission, not
  delete permission. Measure first. Show candidate ID, path, size, risk,
  evidence phrase, and regeneration cost. Default to Trash and state that
  capacity is not free until Trash is emptied. Revalidate before mutation.
  Category-level approval after a frozen ID list can execute in the same turn.
  That last rule conflicts with DevSweep's saved-plan + live digest + `--confirm`
  chain, so this skill must not copy it.
- **win-disk-cleaner**: Broad natural-language triggers, including Chinese
  (`C盘清理`, `释放空间`). Workflow starts with situation assessment. The
  package then ships an admin PowerShell cleaner for WinSxS, hibernation,
  restore points, and recycle bin. DevSweep forbids elevation, Docker, and
  ad-hoc cleanup scripts.
- **windows-disk-cleanup**: Hard split between Discover (dry-run scripts) and
  Execute (`-Execute` only after approval). Presents per-category sizes.
  Execute uses `Remove-Item`, `npm cache clean --force`, and a permanent-delete
  fallback for long paths. Timestamp restoration is a local script concern, not
  a DevSweep contract.
- **storage-cleanup**: Parallel read-only `du` of known cache roots, then a
  SAFE/ASK/KEEP table, then official CLIs or `rm -rf`. Useful report taxonomy.
  Unsafe as an executor: home-directory `du`, `rm -rf`, and `sudo rm -rf`
  suggestion.
- **dry-run-first**: Preview, confirm the preview matches intent, then mutate.
  Maps onto DevSweep `preview` + digest, not onto PowerShell `-WhatIf`.

## Synthesis

- `keep`: audit is not execute; evidence-backed recommendation table;
  dry-run/preview before any side effect; Chinese and English triggers; Trash
  vs reclaimed capacity distinction.
- `adapt`: replace ad-hoc `du`/PowerShell scanners with DevSweep five-mode CLI;
  map SAFE/ASK/KEEP onto Cleanup Target risk, Inspect Only, and Estimated
  Recoverable classes from `CONTEXT.md`.
- `reject`: admin scripts; `rm -rf` / `Remove-Item`; Docker/WSL/WinSxS/hiberfil
  cleanup; software uninstall; same-turn execute after category approval;
  BleachBit/WizTree as the cleanup executor; 向阳乔木 copyright on a product skill.
- `invent`: bind the agent to DevSweep Clean/Analyze/Optimize/Status inspect
  commands; recommend-only default that never emits execute argv; Production
  package under `skills/devsweep-inspect/`.

## Created skill advantages

- Design advantage: the skill uses DevSweep's existing scan → untrusted plan →
  preview digest chain instead of composing cleanup commands.
- Design advantage: recommendation output is required to use `CONTEXT.md` terms
  and Estimated Recoverable classes.
- Hypothesis: Grok description auto-discovery is weaker until `AGENTS.md`
  wiring. Install proof remains missing evidence.
- Validated advantage: trigger eval 13/13 and output fixture eval 2/2 on
  2026-09-01.

## Missing evidence

- skills.sh install counts are telemetry, not ratings.
- SkillsMP query results were mostly off-topic; no fair SkillsMP ranking for
  this job.
- No provider-backed comparison that this skill produces better advice than
  the inspected candidates.
- Unified `--strict` dual-catalog run did not complete because `npx` launch
  failed from Python on Windows.
- Install proof and human review: missing evidence.
