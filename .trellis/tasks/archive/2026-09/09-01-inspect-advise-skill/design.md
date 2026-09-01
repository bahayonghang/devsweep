# Design — Agent CLI inspect-and-advise skill

## Architecture

This parent does not ship product Rust code. It ships an agent skill package
and repo discovery pointers.

```text
skills/devsweep-inspect/          canonical Production package
AGENTS.md                         durable routing for repo agents
code_map.md                       navigation entry
docs/agents/                      optional short pointer; no second workflow
```

Child `09-01-inspect-skill-package` owns every file under
`skills/devsweep-inspect/`. Child `09-01-inspect-skill-discovery` owns
`AGENTS.md`, `code_map.md`, and a docs pointer. The parent owns integration
review only.

## Boundaries

| Layer | May do | Must not do |
|---|---|---|
| Skill workflow | Run DevSweep inspect CLI; write observation/plan JSON the user asked for; recommend | Execute, uninstall, mutate protection, compose shell cleanup |
| DevSweep CLI | Existing five-mode commands | New flags or parser changes |
| Discovery | Point agents at the canonical package | Duplicate `SKILL.md` in `.grok/skills/` |

## CLI data flow

1. Preflight: locate `devsweep` (`cargo run --locked --bin devsweep --` in this
   repo, or an installed binary). Refuse if the binary cannot run.
2. Route the user request:
   - disk / project caches / global caches → `clean scan`
   - tree usage without cleanup authority → `analyze scan`
   - Windows maintenance catalogue → `optimize list`
   - machine snapshot → `status snapshot`
3. Prefer `--format json` and exclusive `--output <new-file>` for machine
   documents. Do not use `-` as stdout for JSON files.
4. Build a recommendation report from the Scan Report or list/snapshot JSON.
   Selected Cleanup Targets stay recommendations. A `clean plan` file is
   allowed only when the user asked to save a plan, and it remains untrusted.
5. Stop. If the user later asks to execute, tell them this skill does not
   execute and point at `clean preview` / `clean execute` with digest and
   `--confirm`. Do not run those commands from this skill.

## Output contract

The recommendation report must include:

- inspect command actually run
- roots and scope
- Cleanup Targets with evidence, risk, and Estimated Recoverable class
- Inspect Only rows called out, including Cargo home
- Optimize catalogue rows as list facts, not completed maintenance
- explicit next step: user approval required before any execute path

Use `CONTEXT.md` terms. Do not write "junk file", "space freed", or "partial
plan" for a Scan Preview.

## Compatibility

- No CLI schema change.
- No gitignored `.agents/` skill copy in this MVP. Agents in this repo load
  `AGENTS.md`.
- Qiaomu Production layout is required. Owner is DevSweep/`bahayonghang`.

## Trade-offs

- Recommend-only vs gated execute: execute stays out. A later skill can own
  the confirm chain without mixing inspect advice with side effects.
- `skills/` vs `.grok/skills/`: user asked for `skills/`. Auto-discovery from
  Grok description scanning is weaker until a future discovery change.
- Product name `devsweep-inspect` vs `qiaomu-devsweep-inspect`: this is a
  product skill, not a Qiaomu-branded public package.

## Rollback

Delete `skills/devsweep-inspect/` and revert discovery edits in `AGENTS.md`
and `code_map.md`. No runtime state is created beyond optional JSON files the
agent writes during a live inspect session.
