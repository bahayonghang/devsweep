# Inspect-advise skill discovery wiring

## Goal

Make repo agents load `skills/devsweep-inspect/SKILL.md` when the user asks to
inspect this machine or wants cleanup or optimization advice.

## Confirmed facts

- This child starts only after `09-01-inspect-skill-package` is archived and
  `skills/devsweep-inspect/SKILL.md` exists.
- Root `AGENTS.md` is always applied. Grok does not auto-scan `skills/`.
- `.agents/` is gitignored. This child must not depend on gitignored copies.
- `code_map.md` is the navigation start point for repo-wide search.

## Requirements

- R1: Add an Agent skills subsection in `AGENTS.md` (outside the Trellis
  managed block) that names `devsweep-inspect`, the path
  `skills/devsweep-inspect/SKILL.md`, natural triggers, and the execute
  exclusion.
- R2: Add a `code_map.md` entry for `skills/devsweep-inspect/`.
- R3: Add a short pointer in `docs/agents/` or the root README that the
  inspect-and-advise workflow lives in that skill. Do not duplicate the
  playbook.
- R4: Do not create `.grok/skills/devsweep-inspect` or a second `SKILL.md`.

## Acceptance Criteria

- [x] AC1 (R1, R2): `AGENTS.md` and `code_map.md` contain the canonical path
      and the recommend-only boundary.
- [x] AC2 (R3, R4): Docs contain one pointer. The diff has no extra `SKILL.md`
      and no files under `.grok/skills/`.

## Out of scope

- Editing files inside `skills/devsweep-inspect/` except to fix a broken
  relative link if discovery text requires it.
- User-level `~/.grok/config.toml` changes.
- Publishing or `npx skills add`.
