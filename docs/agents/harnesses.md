# Five-harness matrix

Verification date: **2026-09-07**. Refresh that date if you re-probe. This page
records how five tools load this repository. It is not a quality ranking and
not a price promise.

A **harness** is a tool, permission, and context environment (Claude Code,
Codex, Grok Build, Kimi Code, OMP / Oh My Pi). A **model** is a reasoning and
cost choice inside a harness. Do not call OMP or Kimi “cheaper models”; they
are harnesses. Low-cost **models** are optional executors only after facts,
file scope, and checks are frozen.

Shared product rules live in root `AGENTS.md`. Tool-local notes here record
entry points and evidence, not a second safety contract.

## Adapter kinds

| Kind | Meaning in this repo | Do not infer |
| --- | --- | --- |
| Canonical | Files this project owns as the contract: `AGENTS.md`, `skills/devsweep-inspect/`, and `CLAUDE.md` as `@AGENTS.md`. | That every tool uses a native adapter directory. |
| Compatibility | A tool reads those shared files (and often `.agents/skills` or `.claude/`) without a Trellis-generated native root. | That missing `.grok` / `.kimi-code` / `.omp` means the tool cannot work here. |
| Native-Trellis | Directories `trellis platforms` currently lists for this project: `.claude` (Claude Code), `.codex` (Codex; also writes `.agents/skills`), `.reasonix`. | That host CLI `0.7.0-beta.3` extra init flags are already configured, or that those files are git-tracked. |

Do **not** create `.grok/`, `.kimi-code/`, or `.omp/` for symmetry. Native
Trellis adapters for those tools are a later review, not this alignment.

## Project vs global boundary

In-repo, versioned contract:

- `AGENTS.md` (and `CLAUDE.md` → `@AGENTS.md`)
- `skills/devsweep-inspect/` as the inspect skill source
- `just install-skill` / `just check-skills` copy and compare discovery trees
  under `.agents/skills` and `.claude/skills` only

Gitignored local adapters (present on this machine, not the source of truth):

- `.agents/`, `.claude/`, `.codex/`, `.reasonix/`

Do not change user-global configs, hooks, models, or skills to “finish”
alignment. `just check-skills` after skill-integrity: discovery copies of
`devsweep-inspect` must match `skills/devsweep-inspect`.

## Trellis version

Project `.trellis/.version` is **0.6.12**. The host `trellis` CLI on this
machine is **0.7.0-beta.3** and prints
`Trellis update available: 0.6.12 → 0.7.0-beta.3`. That line is version
status, **not** upgrade authorization. Do not run `trellis update`.

`trellis platforms` on 2026-09-07 listed Claude Code → `.claude`, Codex →
`.codex` (also `.agents/skills`), Reasonix → `.reasonix`. It does not list
Grok, Kimi, or OMP native roots.

## Planning vs bounded execution

Use a **strong model** for planning, cross-file review, safety/skill
semantics, and any change that could alter permissions, task phase, or file
scope.

A **bounded low-cost model** may edit only after inputs, outputs, file paths,
and regression checks are frozen. Deterministic scripts are preferred over a
cheap model when the work is mechanical.

If a bounded executor hits permission, semantic, or cross-module issues
**twice**, escalate to the strong reviewer. Do not pile patches, widen scope,
or relax checks.

| Work | Strong planning / review | Bounded execution (after freeze) | Upgrade immediately |
| --- | --- | --- | --- |
| Harness docs / this matrix | Claude Code or Codex strong review of conflicts | Fill a row from already-recorded facts, links, and dates | Runtime schema differs, hooks would be enabled, or native adapters would be added |
| CLI / bilingual command text | Claude Code / Codex freeze the command and safety facts | Path and wording edits against that freeze | Any CLI/API or permission change |
| Skill copy / hashes | Claude Code / Codex on semantics; Grok inspect as discovery counter-evidence | File sync and hash checks without rewriting cleanup flow | Rewrite of inspect execute rules or user-global installs |
| Gate / lock / release | Codex strong review on Windows/PowerShell roots | Named recipe or metadata edits already designed | Exit-code or lock semantics unclear |

## Evidence levels

Do not collapse these four levels. Only level 4 may claim the matching
behavior actually ran.

1. **Official capability** — vendor docs (URLs below were opened during the
   2026-09-07 audit).
2. **Local files present / ignored** — paths on disk in this clone.
3. **Actual discovery** — a tool listed or loaded those files in a real
   inspect/session.
4. **Actual execution** — a session used the capability (hooks, agents,
   models, TUI, hosted CI). Mostly **UNVERIFIED**.

Installed CLI versions observed 2026-09-07 (install snapshot, not proof that
the current desktop session is that binary):

| Tool | Observed `--version` |
| --- | --- |
| Claude Code | `2.1.263` |
| Codex CLI | `codex-cli 0.153.4` |
| Grok | `grok 1.0.22 (8f40483ca2a5)` |
| Kimi | `0.41.0` |
| OMP | `omp/18.1.12` |
| Trellis host CLI | `0.7.0-beta.3` |

## Claude Code

### 1. Official capability

[Features overview](https://code.claude.com/docs/en/features-overview) and
[memory / CLAUDE imports](https://code.claude.com/docs/en/memory): project
`CLAUDE.md`, `@` imports, skills, named subagents, and hooks are documented
product surfaces.

### 2. Local files present / ignored

Present and gitignored: `.claude/` (`agents/`, `commands/`, `hooks/`,
`skills/`, `settings.json`). Canonical tracked files: `CLAUDE.md` (`@AGENTS.md`),
`skills/devsweep-inspect/`. Discovery copy: `.claude/skills/devsweep-inspect/`
via `just install-skill`.

### 3. Actual discovery

Project `CLAUDE.md` exists as an `@AGENTS.md` import. Local `.claude` assets
exist. `just check-skills` is the in-repo integrity check for the discovery
copy. A fresh Claude Code session handshake (which files the model actually
loaded, which project hooks fired) is **not** recorded here.

### 4. Actual execution

**UNVERIFIED** for a fresh Claude Code session, project hook firing, and
named subagent dispatch. Claude doctor health is not execution of this
repo’s instructions.

## Codex

### 1. Official capability

[AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md),
[skills](https://learn.chatgpt.com/docs/build-skills),
[subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents),
[Windows sandbox](https://learn.chatgpt.com/docs/windows/windows-sandbox):
layered project rules, skill discovery, configurable subagents, and an
optional Windows sandbox. Product support for sandbox is not evidence that
this session used it.

### 2. Local files present / ignored

Present and gitignored: `.codex/` (`agents/`, `hooks/`, `skills/`,
`config.toml`, `hooks.json`) and `.agents/skills/`. Canonical: root
`AGENTS.md`. Codex/Trellis also writes `.agents/skills` for other tools.

### 3. Actual discovery

The 2026-09-07 audit session received project `AGENTS.md` / Trellis stage
context. `trellis platforms` maps Codex → `.codex` and `.agents/skills`.
Individual hook paths were not independently proven.

### 4. Actual execution

A Codex-style / Cursor session on 2026-09-07 ran planning review and this
alignment edit. That environment was **danger-full-access / sandbox
disabled**. Do **not** call it sandboxed isolation, even though Codex can
use Windows sandbox in other setups. Fresh Codex CLI sessions and each
provider hook remain **UNVERIFIED**.

Project Codex hooks stay optional local scaffolding: do not assume
`.codex/hooks.json` is active unless user-level Codex config enables the
hook and it has been approved (`AGENTS.md`).

## Grok Build

### 1. Official capability

[Skills and compatibility](https://docs.x.ai/build/features/skills-plugins-marketplaces)
and [plan mode caveats](https://docs.x.ai/build/features/plan-mode):
compatibility reads of AGENTS/Claude-style files, skills, subagents, and
plan/permissions. Plan mode gates **edit** tools; **bash stays writable**.
Child agents are not automatically covered by the parent plan edit gate.
Every delegation still needs an explicit read-only or allowed-file scope.

### 2. Local files present / ignored

No project `.grok/` directory (and none should be created). Grok can still
consume canonical `AGENTS.md` / `CLAUDE.md` and gitignored `.agents/skills`
plus `.claude/` compatibility trees.

### 3. Actual discovery

`grok inspect --json` on 2026-09-07 (`grok 1.0.22`): `projectTrusted=true`;
project instructions included repo `Agents.md` and `Claude.md` (Windows
path case); `devsweep-inspect` with `source.type=project` and
`source.path` → `.agents/skills/devsweep-inspect/SKILL.md`; Trellis skills
from `.agents/skills/`; `trellis-implement` and `trellis-research` from
`.claude/agents/`. Project `.claude` hook events listed included
`session_start`, `user_prompt_submit`, `pre_tool_use`, `post_tool_use`.
Several listed `.claude` hook entries were marked disabled in that dump.
This is compatibility **discovery**, not hook execution.

### 4. Actual execution

**UNVERIFIED** for Grok hook firing, subagent runs, and plan-mode
enforcement. Do not treat plan mode as a read-only shell.

## Kimi Code

### 1. Official capability

[Agents](https://www.kimi.com/code/docs/en/kimi-code-cli/customization/agents.html),
[hooks](https://www.kimi.com/code/docs/en/kimi-code-cli/customization/hooks.html),
[skills](https://www.kimi.com/code/docs/en/kimi-code-cli/customization/skills.html):
custom agents, hooks, and skill locations including project
`.kimi-code/skills/` **and** `.agents/skills/`. Foreign `model` fields on
imported Claude agent files are ignored; they are not a portable cheap-model
router.

### 2. Local files present / ignored

CLI installed (`kimi 0.41.0`). No project `.kimi-code/` (do not create it).
Shared discovery may still use `.agents/skills/` per official skill
locations. That is compatibility, not a native-Trellis adapter.

### 3. Actual discovery

**UNVERIFIED** in a fresh Kimi session for this repo. Do not reuse older
templates that claimed Kimi has “no hooks.”

### 4. Actual execution

**UNVERIFIED**. Pick a cost tier only in a session whose runtime schema is
confirmed. Do not assume Claude agent `model` fields apply.

## OMP (Oh My Pi)

### 1. Official capability

[Task agent discovery](https://github.com/can1357/oh-my-pi/blob/main/docs/task-agent-discovery.md),
[context files](https://github.com/can1357/oh-my-pi/blob/main/docs/context-files.md),
[skills](https://github.com/can1357/oh-my-pi/blob/main/docs/skills.md):
shared rules/skills and task-agent role/model routing. Official docs are
capability, not a project handshake.

### 2. Local files present / ignored

CLI installed (`omp/18.1.12`). No project `.omp/` native roles (do not
create that directory). Shared `AGENTS.md` / `.agents/skills` remain the
intended compatibility path.

### 3. Actual discovery

**UNVERIFIED** in a fresh OMP session against this clone.

### 4. Actual execution

**UNVERIFIED**. A fixed role may later run copy/docs/deterministic checks
on a lower-cost **model**; validate role resolution and permissions first.
OMP itself is not “the cheap model.”

## Explicit UNVERIFIED

Do not describe the five-harness alignment as a five-tool runtime
acceptance. Still **UNVERIFIED**:

- Provider hook execution (Claude, Codex, Grok, Kimi, OMP)
- Fresh Kimi, OMP, and Claude Code project sessions
- Grok hook and agent execution (inspect ≠ run)
- Native interactive TUI (bare `devsweep` on real TTYs)
- Hosted CI of the branch that carries this documentation change
- Windows sandbox isolation for Codex (this session had sandbox disabled)

## How to refresh

Read-only probes used for this page (no paid batch jobs, no `trellis update`,
no global config edits):

```powershell
trellis platforms
trellis --version
Get-Content .trellis/.version
just check-skills
grok inspect --json
```

Re-open the official URLs above if a vendor surface changed. Keep the four
evidence levels distinct when you update a row.
