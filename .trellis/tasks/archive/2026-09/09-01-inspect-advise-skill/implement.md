# Implement — inspect-advise skill umbrella

This parent coordinates contracts and final review. Do not start it as the
implementation task.

## Child order

1. Complete and archive `09-01-inspect-skill-package`.
2. Start `09-01-inspect-skill-discovery` only after the package exists at
   `skills/devsweep-inspect/SKILL.md`.
3. Parent review after both children are archived.

## Umbrella checks

- Package path is `skills/devsweep-inspect/` with one discoverable `SKILL.md`.
- `validate_skill.py` and `trigger_eval.py` reports exist under the package
  `reports/` directory.
- `AGENTS.md` and `code_map.md` point at that path.
- The skill workflow contains inspect commands and excludes execute,
  uninstall, and shell-composed cleanup.
- Domain terms match `CONTEXT.md`.

## Rollback

Revert the two children. Keep this parent in `planning` or `in_progress`
until the integration review file exists.

## Integration review file

Write `.trellis/tasks/09-01-inspect-advise-skill/acceptance.md` with PASS or
FAIL, child archive paths, validation commands, and any `UNVERIFIED` items.
Overall PASS requires no completion-required `UNVERIFIED`.
