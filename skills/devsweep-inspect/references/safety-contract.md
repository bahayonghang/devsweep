# Safety contract

A cleanup or optimization request is permission to inspect. It is not
permission to mutate the machine.

## Allowed

- Run DevSweep inspect commands listed in [cli-playbook.md](cli-playbook.md).
- Write a new observation JSON or untrusted Cleanup Plan file when the user
  asked to save one. Use exclusive create-new `--output`.
- Recommend Cleanup Targets with evidence, risk, and Estimated Recoverable
  class.
- Tell the user that later execution needs `clean preview`, a live `sha256:`
  digest, and `--confirm`. Omit execute argv from the recommendation report.

## Forbidden

- `clean execute`
- `optimize run`
- `software uninstall`
- `clean protect add` and `clean protect remove`
- `rm`, `rmdir`, `del`, `Remove-Item`, `Clear-RecycleBin`
- Shell strings that combine a program and arguments for cleanup
- Permanent delete, Docker cleanup, Cargo home cleanup, elevation, UAC
- Treating a preview digest as an execute grant
- Treating an Optimize Settings launch as completed maintenance
- Treating DevSweep default ranking as user approval

## Inspect Only and exclusions

Mark these rows `inspect-only` or `exclude`. Do not recommend them for cleanup:

- Cargo home, credentials, installed binaries, registry internals, git cache
  internals
- Docker data
- Paths on the protection list
- Incomplete sizes presented as exact reclaim

## Capacity language

Use Estimated Recoverable classes: verified, partial-lower-bound, unknown.
Do not write "space freed" or "released space" for a recommendation.
Trash-backed cleanup does not free capacity until the user empties trash.
That statement belongs in the next-step note, not as a reclaim promise.
