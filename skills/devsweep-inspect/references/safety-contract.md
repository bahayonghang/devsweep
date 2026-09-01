# Safety contract

An inspect or advice request is permission to observe. It is not permission
to mutate the machine.

A cleanup request is permission to build and display a selectable list. It
is not execute authority until the user confirms those listed ids.

## Allowed

- Run DevSweep inspect commands listed in [cli-playbook.md](cli-playbook.md)
  using the globally installed binary from
  [../scripts/resolve_devsweep.py](../scripts/resolve_devsweep.py).
- Write a new observation JSON or untrusted Cleanup Plan file. Use exclusive
  create-new `--output`.
- Recommend Cleanup Targets with evidence, risk, and Estimated Recoverable
  class.
- After a displayed list and an explicit user confirmation of named ids, run
  `clean preview` and then `clean execute` with the live `sha256:` digest and
  `--confirm`. See [confirmed-clean.md](confirmed-clean.md).

## Forbidden

- `clean execute` in the same turn as the first cleanup list
- `cargo run` of this repository, or any `target\\debug` / `target\\release`
  DevSweep binary from this checkout
- `optimize run`
- `software uninstall`
- `clean protect add` and `clean protect remove`
- `rm`, `rmdir`, `del`, `Remove-Item`, `Clear-RecycleBin`
- Shell strings that combine a program and arguments for cleanup
- Permanent delete, Docker cleanup, Cargo home cleanup, elevation, UAC
- Treating a preview digest as a user execute grant
- Treating an Optimize Settings launch as completed maintenance
- Treating DevSweep default ranking as user approval
- Selecting this repository's own tree unless the user names those ids after
  seeing them

## Inspect Only and exclusions

Mark these rows `inspect-only` or `exclude`. Do not select them for cleanup:

- Cargo home, credentials, installed binaries, registry internals, git cache
  internals
- Docker data
- Paths on the protection list
- Unresolved provider paths
- Incomplete sizes presented as exact reclaim

## Capacity language

Use Estimated Recoverable classes: verified, partial-lower-bound, unknown.
Do not write "space freed" or "released space" for a recommendation or an
execution report. Trash-backed cleanup does not free capacity until the user
empties trash.
