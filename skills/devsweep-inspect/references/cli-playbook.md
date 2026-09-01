# CLI playbook

Keep program and argv separate. Do not build a shell cleanup string.

## Program

In this repository:

```text
cargo run --locked -p devsweep-cli --bin devsweep --
```

If `devsweep --version` works, the agent may use that binary instead.

Add `--language zh-CN` or `--language en` only for human output. Do not pass
`--language` on JSON or plan-producing commands.

`--output <FILE>` creates a new file. Do not overwrite. Do not use `-` as
stdout for JSON files.

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
| Save selected Cleanup Plan | `clean plan --observation <FILE> --select <ID> [--select <ID> ...] --output <FILE>` |
| Preview a saved Cleanup Plan | `clean preview --plan <FILE> --format json` |

Repeat `--root` to keep several roots in command-line order.

`clean plan` is allowed only when the user asked to save a plan file. The
saved document stays an untrusted Cleanup Plan.

## Commands this skill must not run

```text
clean execute
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

Copy the exact program and argv into the recommendation report. Quote paths
that contain spaces as separate argv items, not as a concatenated command
line.
