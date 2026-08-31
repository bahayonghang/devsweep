# CLI Migration

The redesigned command surface is intentionally breaking. Old roots and
options are rejected rather than retained as aliases, and old reports or plans
never become new execution authority.

| Old invocation | New invocation or disposition |
| --- | --- |
| `devsweep tui` | Run bare `devsweep` from an interactive stdin/stdout TTY. The old root is rejected. |
| `devsweep scan ROOT... [--global|--projects] [--json]` | Use `devsweep clean scan --root ROOT... --scope <projects|global|all> --format json`. Old scan documents are not executable authority. |
| `devsweep scan --rescan-target ID ROOT` | Use `devsweep clean scan --root ROOT --scope projects --rescan-target ID`. |
| `devsweep inventory ROOT [--json]` | Use `devsweep analyze scan --root ROOT --format json`; the schema is replaced. |
| `devsweep clean --plan P` | Rescan and replan, then use `devsweep clean preview --plan NEW_P`. There is no old-plan conversion. |
| `devsweep clean --plan P --execute` | Rescan, create a saved plan, preview it, then use `devsweep clean execute --plan NEW_P --preview-digest DIGEST --confirm`. |
| `devsweep clean --audit-log P` | `--audit-log` is removed. `P` is not moved, imported, converted, or searched by `history`. |
| `devsweep protect list|add|remove` | Use `devsweep clean protect ...`; mutations require explicit `--path` and `--confirm`. |
| `devsweep rules` | Use `devsweep clean rules list`. |

New roots with no old equivalent:

| Invocation | Disposition |
| --- | --- |
| `devsweep software {inventory,plan,preview,uninstall}` | Separate Software domain. Only exact current-user MSIX identities can execute. Every MSI remains visible and manual. |
| `devsweep optimize {list,plan,preview,run}` | Closed eight-entry catalogue. `dns.flush` runs here; Settings ids launch a frozen URI; guidance ids cannot be planned. |
| `devsweep status {snapshot,live}` | Read-only metrics. `status live --format human` requires a TTY; NDJSON is the non-interactive stream. |
| `devsweep history {list,show}` | Read-only inspection of the fixed V1 audit stores. Unknown or newer schema versions are preserved and never executed. |

TUI and the packaged Tauri app follow the same mode set. Unavailable
capabilities are omitted or labelled unsupported; they are not placeholders or
compatibility aliases.

Mutating operations append only to the fixed stores under
`%LOCALAPPDATA%\DevSweep\audit\v1\<domain>.jsonl`. A user-supplied legacy audit
file and the legacy default `%APPDATA%\devsweep\audit.jsonl` remain byte-for-byte
untouched. There is no automatic discovery, import, conversion, or replay of
either legacy source.

Saved-plan commands reject stdin and `-`, never overwrite output, and require a
new plan built under the current schema. Execution additionally requires the
matching digest from a live preview and the non-interactive `--confirm` flag.

Rollback is the last accepted local release binary and package. New plan and
audit schema versions are not read as old versions. Local NSIS packaging is
unsigned and current-user; this integration task does not sign, push, publish,
or create a GitHub release.
