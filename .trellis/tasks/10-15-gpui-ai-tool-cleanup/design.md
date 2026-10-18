# C15 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Use provider documentation/source and current supported Windows layouts, never translate macOS paths mechanically. No broad ~/.claude or ~/.codex deletion. C03 owns ordinary cache registry integration; C15 owns sensitive AI-data classification and opt-in recovery. Keep session/worktree operations distinct from Clean cache rules. This child only emits typed installed-tool handoff identity; actual tool uninstall is C04/C05 work and native integration waits for C11/C12.

## Ownership

- `crates/devsweep-core/src/rules/ and scan/ AI adapters`
- `new narrowly scoped AI session/worktree recovery modules`
- `crates/devsweep-cli/src/application/commands/clean.rs and child modules`
- AI CLI parser additions after C01; C11 exclusively owns the native AI review UI.

These paths identify responsibility, not permission to refactor every file.
One owner edits a shared file at a time. Workers are not alone in this repository
and must not revert other workers' changes. Main session owns cross-child design,
spec updates and integration decisions.

## Data flow and compatibility

CLI request -> domain observation/plan -> live validation -> explicitly confirmed execution where supported -> durable audit/read-back -> typed output. Discovery never mutates system state.

Freeze a provider-specific transaction matrix before enabling sensitive operations:
unknown active ownership or unobservable index changes refuse execution; failure
before durable recovery preserves the original state; later failures retain original
files and provider metadata through the permitted recovery mechanism and report
partial/unknown outcomes without retry. Restore uses a saved restore intent through
the same preview/execute authority and refuses destination/index drift. A backup is
not an exception to the parent's permanent-deletion exclusion. Unsupported session
or worktree mechanisms remain explicitly inspect-only.

Preserve existing supported schemas, command semantics, refusal and cancellation.
Version incompatible shapes and consult the parent CLI proposal. Rust native views
call shared services, rather than parsing CLI text or cloning domain logic.
Follow the latest-compatible dependency policy; any older required pin or compiler
compatibility change is documented and tested.

## Risks and rollback

Identity drift, partial observations, late worker results and broadening authority
are relevant failure cases. GPUI-specific native accessibility/IME/graphics are
unproven until C09. Native uninstaller/helper/provider behavior cannot be inferred
from fixture success alone. Keep operation-specific close/cancel semantics.

Retain existing CLI/Tauri paths while this child is developed. Roll back isolated
adapter changes without changing unrelated work or historical evidence. Installation,
publishing and Tauri retirement are not implicit consequences of passing tests.

## Technical freeze (2026-10-17)

Authority: this section, `prd.md`, `implement.md`, parent
`research/cli-contract-proposal.md` (`clean ai` leaves), and
`research/mole-014-rea.md`. Mole 014 does not name Claude, Codex, or Cursor
session paths. Do not translate a macOS layout into a Windows default. Do not
scan or modify this machine's real `.claude`, `.codex`, or Cursor profile.
Tests pass an explicit absolute `--home`.

C03 remains the owner of `windows.cursor.cache` and of the coverage-only ids
`ai.claude.cache`, `ai.codex.cache`, and `ai.cursor.sessions`. Generic
`clean scan` must not select AI sessions, settings, credentials, memory,
models, or worktrees. This child adds `clean ai` only.

### Command

Additive leaves, same confirmation rule as Clean:

- `clean ai scan --home <ABS>` writes an observation. It does not mutate.
- `clean ai plan --scan <FILE> --class <ID>` writes a plan. It does not mutate
  the home.
- `clean ai preview --plan <FILE>` prints a live `sha256:` digest. It does not
  mutate the home.
- `clean ai execute --plan <FILE> --preview-digest sha256:<HEX> --confirm`
  is the only home mutation.
- `clean ai recovery-list --home <ABS>` lists recovery bundles. It does not
  restore.
- `clean ai restore-plan --home <ABS> --bundle <ID>` writes a restore intent.
  Restore then uses the same `preview` and `execute` path.

Missing plan, digest, or confirm exits 2. Changed identity exits 3. Unavailable
source or permission exits 4. A truthful partial result exits 5. Uncertain
dispatch or persistence exits 6. Cancel before dispatch exits 130. No
`--execute` flag and no permanent-delete flag. Human and JSON use the existing
V1 envelope. JSON codes stay English. Add the new help strings to
`resources/i18n/en.json` and `resources/i18n/zh-CN.json`. Regenerate
`docs/reference/cli.md` and `docs/zh/reference/cli.md` from the Clap tree only
if the existing manifest test requires it.

### Providers and classes

The closed provider set is `claude-code`, `codex`, `cursor`. No further tools.
Each home is a fixture the test creates. Production defaults, used only when
`--home` is omitted, are `%USERPROFILE%\.claude`, `%USERPROFILE%\.codex`, and
`%APPDATA%\Cursor`. A missing default is `absent`, not an error, and is not a
reason to scan the profile. Cursor Electron cache children stay on
`windows.cursor.cache`. This command does not select them.

Closed class ids:

| Class | Selectable | Rule |
| --- | --- | --- |
| `regenerable_cache` | unselected until the plan names it | only direct children of `cache/` named `regen-*` |
| `superseded_version` | unselected until the plan names it | a directory `versions/<name>` whose name is not the single line in `versions/current.txt` |
| `session_archive` | opt-in plan only | only `sessions/<id>.json` with the closed version below |
| `worktree` | opt-in plan only | only a directory under `worktrees/` |
| `installed_tool` | never | typed identity `{provider, version, install_kind}` from `install.json` when that file is `{"version":"<token>"}`; otherwise version `unknown`. No uninstall and no deletion of the install directory |

Never select `settings`, `credentials`, `memory`, `skills`, `mcp`, `models`,
`projects`, the current version directory, or a session id named by the single
line in `sessions/active`. Unknown files are `unsupported_version` and are not
targets. A directory newer than another directory is not authority.

### Session file

The only executable session version is JSON object
`{"version":1,"id":"<id>","active":false,"body":"<text>"}` with `deny_unknown_fields`.
Any other bytes, `active:true`, or an id equal to `sessions/active` is refused
before a copy or a move. This is not a claim that current Claude, Codex, or
Cursor release files use this schema. Those files stay `unsupported_version`.

### Mutation

The only file mutation is the existing Clean move-to-trash action. Do not call
`git worktree remove`, `Remove-Item`, or any delete API. Program and argv stay
separate when a Git read is required: `git` with argument list
`-C`, `<worktree>`, `status`, `--porcelain=v1`, `-uall`, and a separate
`rev-list` read for unpushed commits. Do not compose a shell string.

A worktree plan is legal only when a disposable repository proves all of:
registered worktree, clean tracked status, no untracked or ignored files, no
unpushed commits, no submodule, no shared git dir inside the worktree, and no
active owner. Age is not enough. If Git is missing or any check is unknown,
the row is inspect-only. Dirty, untracked, ignored, unpushed, submodule,
shared, and active rows stay protected.

Before the trash move, write a recovery bundle under `<home>/.devsweep-recovery/<id>/`
containing the original bytes and a sha256. The bundle write is the durable
start. If it fails, the home is unchanged. After a successful bundle write,
a failed or interrupted move leaves the original in place or reports unknown
without a second attempt. Restore refuses when the destination exists, the
bundle hash differs, or the live file changed. A bundle is not permission to
delete permanently.

Unknown active ownership, a lock that cannot be acquired, or an index that
changes between plan and preview refuses before the bundle write. Do not retry.

### Tests

Spawn `env!("CARGO_BIN_EXE_devsweep")` for preview parity, missing confirm,
stale digest, unsupported session bytes, protected adjacent files, and a
restore refusal after an external edit. Core tests cover the class matrix on
temp homes for all three providers, including locked and malformed fixtures.
One worktree fixture is a disposable repository. Assert protected classes are
not targets. Assert a refused execute leaves a recorded hash of the home.
Do not check an acceptance box whose case is absent from the saved log.
Worktree removal that would require `git worktree remove` stays unimplemented;
the trash path above is the only removal. If that path cannot keep Git metadata
consistent, execute refuses with `deletion_policy` and AC4 records that refusal
instead of a pass.

Do not edit `desktop/`, `desktop-gpui/`, root `Cargo.toml`, `justfile`,
`.github/`, or `.trellis/scripts/`. Do not commit. Cargo tests use
`--target-dir dist/ci-target`. Do not export `CARGO_TARGET_DIR`. Do not set
`HOME`, `CARGO_HOME`, or `RUSTUP_HOME`. Do not download Rust 1.88.0.
