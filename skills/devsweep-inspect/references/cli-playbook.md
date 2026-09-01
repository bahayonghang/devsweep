# CLI playbook

Keep program and argv separate. Do not build a shell cleanup string.

## Program

Resolve once per session:

```text
python scripts/resolve_devsweep.py --repo-root <repo>
```

Use that absolute path as argv[0]. Never:

```text
cargo run --locked -p devsweep-cli --bin devsweep --
target\debug\devsweep.exe
target\release\devsweep.exe
```

Add `--language zh-CN` or `--language en` only for human output. Do not pass
`--language` on JSON or plan-producing commands.

`--output <FILE>` creates a new file. Do not overwrite. Do not use `-` as
stdout for JSON files.

Do not default `--root .` when the current directory is this repository.
Pass explicit roots. Repeat `--root` to keep several roots in command-line
order.

## Intent map

| User intent | Argv after the program |
|---|---|
| Project and global cleanup candidates | `clean scan --root <PATH> --scope all --format json --output <FILE>` |
| Projects only | `clean scan --root <PATH> --scope projects --format json --output <FILE>` |
| Global providers only | `clean scan --root <PATH> --scope global --format json --output <FILE>` |
| Disk tree without cleanup authority | `analyze scan --root <PATH> --format json --output <FILE>` |
| Windows maintenance catalogue | `optimize list --format json` |
| Machine snapshot | `status snapshot --format json` |
| Recent audit records | `history list --format json` |
| Cleanup rules | `clean rules list --format json` |
| Protection list | `clean protect list --format json` |
| Save selected Cleanup Plan | `clean plan --observation <FILE> --select <ID>... --output <FILE>` |
| Preview a saved Cleanup Plan | `clean preview --plan <FILE> --format json --output <FILE>` |
| Execute after list confirmation | `clean execute --plan <FILE> --preview-digest sha256:<DIGEST> --confirm --format json --output <FILE>` |

`clean plan` / `clean preview` / `clean execute` follow
[confirmed-clean.md](confirmed-clean.md). The saved document stays an
untrusted Cleanup Plan until preview and `--confirm`.

On Windows, batch `--select` so the process argv stays under about 20 000
characters. `scripts/plan_selected.py` does that and skips ids that fail
`validate_plan`.

## Commands this skill must not run

```text
optimize plan
optimize preview
optimize run
software uninstall
software plan
clean protect add
clean protect remove
```

`software inventory` may run only as supporting context. It does not create
uninstall authority.

`status live` is a stream. Prefer `status snapshot` for a one-shot report.

## Recording

Copy the exact program and argv into the recommendation or execution report.
Quote paths that contain spaces as separate argv items, not as a concatenated
command line. Inspect-only reports still omit execute argv.
