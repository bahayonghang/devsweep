# Repository Guidance

This root `AGENTS.md` governs the entire repository. Deeper `AGENTS.md` files
may add or override guidance for their subtrees. Read `./code_map.md` before
broad grep or repo-wide search so navigation starts from the maintained map.

## Commands

- `just ci` is the canonical local gate; it runs format, check, tests, and
  clippy.
- `just dev` runs the CLI binary with
  `cargo run --locked -p devsweep-cli --bin devsweep` (no extra argv).
- Interactive TUI is bare `devsweep` with stdin and stdout TTYs.
- `just tdev` starts the Tauri desktop development window with
  `cd desktop; npm run tauri -- dev`.
- `just build` runs
  `cargo build --locked -p devsweep-cli --bin devsweep`.
- `just release-archive` builds a locked release binary for the rustc host
  triple and writes `dist/devsweep-<triple>.zip`.

## Safety Contracts

- Cleanup dry-run is `clean preview`. Execute only from an explicit saved
  plan plus a live `sha256:` preview digest and `--confirm`. There is no
  `--execute` flag.
- Permanent delete remains disabled in this build and is not a CLI option.
  There is no `--allow-permanent-delete` flag.
- Scanner and model code must only create cleanup plans; do not delete files,
  move paths to trash, or execute external cleanup commands from those layers.
- Command-backed cleanup must keep program and argv separate. Do not compose
  shell command strings for cleanup actions.
- Cargo home is inspect-only. Do not create cleanup actions for Cargo
  credentials, installed binaries, registry internals, or git cache internals.
- Docker cleanup is not part of the current MVP behavior.
- `target/`, `dist/`, and `__pycache__/` are generated outputs and must not be committed.

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues for `bahayonghang/devsweep`; external PRs
are not a triage surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default canonical labels: `needs-triage`, `needs-info`,
`ready-for-agent`, `ready-for-human`, and `wontfix`. See
`docs/agents/triage-labels.md`.

### Domain docs

This repo uses a single-context domain-doc layout. See
`docs/agents/domain.md`.

### Harnesses

Claude Code, Codex, Grok Build, Kimi Code, and OMP (Oh My Pi) share this
file as the project contract. See `docs/agents/harnesses.md` for the
five-harness matrix, adapter kinds, and evidence levels.

### Inspect and advise

When the user asks to inspect this machine, recommend cleanup, or give
optimization advice, read `skills/devsweep-inspect/SKILL.md` and follow it.
Call the globally installed `devsweep` binary; do not `cargo run` this
repository as the cleaner. Inspect stays recommend-only until the user
confirms a displayed cleanup list. After that confirmation, the skill may
run `clean execute` with a live preview digest and `--confirm`. Do not run
`optimize run` or `software uninstall` from that workflow.

## Trellis And Codex

- For backend changes, read `.trellis/spec/backend/index.md` before editing.
- For TUI changes, read `.trellis/spec/frontend/index.md` before editing.
- For desktop React/Tauri changes, read
  `.trellis/spec/desktop-frontend/index.md` before editing.
- Project Codex hooks and agents are optional local scaffolding. Do not assume
  `.codex/hooks.json` hooks are active unless the user-level Codex config
  enables hooks and the hook has been approved.
- Preserve the managed Trellis block below exactly; add project guidance outside
  the marker block.

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
