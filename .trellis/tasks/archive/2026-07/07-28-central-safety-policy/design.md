# Design: Central Safety Policy and Live Revalidation

## Authority boundary

Every mutation passes through one `SafetyPolicy::authorize` entry point after
plan validation and before the runner or trash adapter is reached. Its input is
`ValidatedAction`, never an untrusted DTO. Its successful output is an opaque
`AuthorizedAction` that carries the reconstructed action and fresh live
revalidation evidence; executor internals cannot construct it directly.
`Denial` is typed and includes the matching protection category and normalized
path so TUI, CLI, and audit can explain a safe refusal without guessing.

`07-28-plan-validation` carries the versioned scan-time footprint fact in the
v2 DTO and validates its shape. This task owns how it is captured and compared:
Windows uses stable file identity, Unix uses device/inode, and platforms with a
different stable identifier use an equivalent documented value. A missing,
malformed, or changed expected footprint denies execution and requests rescan.

## Protection evaluation

Policy evaluation is ordered and default-deny:

1. `ProtectedSubtree`, `OwnVcsMetadata`, and `UserProtectionList` deny a
   target equal to, inside, or containing a protected path.
2. `ExactNode` denies equality with or ancestry over volume roots, home,
   Desktop/Documents/Downloads, scan roots, and repository roots, but does not
   reject every legal descendant.
3. `AuthorizedFootprint` must then prove that a project target belongs to its
   validated scan root and rule footprint, or that a global target matches the
   registry's allowed global footprint.

The live check reopens metadata for the target and every ancestor through its
authorized root, rejects new symlink/junction/reparse nodes, verifies expected
marker files, containment, current executable, and audit-log exclusion. Failure
to obtain `current_exe()` or any required identity is a denial, never a pass.

## UserProtectionList (D6)

The persistent config is a versioned JSON document in an OS app-data location:
`%APPDATA%\\devsweep\\protected-paths.json` on Windows,
`~/Library/Application Support/devsweep/protected-paths.json` on macOS, and
`$XDG_CONFIG_HOME/devsweep/protected-paths.json` (or `~/.config`) on Unix.
It contains canonical absolute protected paths and no executable configuration.
Writes use a same-directory temporary file plus atomic rename; malformed,
unreadable, or unwritable policy state denies mutation rather than falling back
to an empty list.

`devsweep protect add <PATH>` requires an existing path and stores its canonical
absolute form. `protect remove <PATH>` first canonicalizes an existing path;
when it no longer exists it matches the normalized absolute representation
against stored canonical entries. `protect list` prints only stored paths and
performs no scan or mutation. All commands are explicit CLI operations, not
implicit behavior of `scan` or `clean`.

## Cargo target scope

Project Rust discovery invokes `cargo metadata --format-version 1 --no-deps`
through the bounded `ProcessRunner` and derives `workspace_root` and
`target_directory`. Display, estimate, footprint, self-exe guard, and the
reconstructed `cargo clean` action use that same resolved target directory.
The action carries `--target-dir <resolved>` so execution does not silently
change scope. The safety check runs metadata again immediately before execution;
a changed target/workspace mapping denies and requests rescan/reconfirmation.
Metadata failures downgrade the rule to a local trash candidate only when that
candidate independently satisfies the authorized-footprint contract; otherwise
it is inspect-only with an explicit diagnostic.

## Integration and rollback

The runner dependency is explicit: cargo metadata must use the bounded
`ProcessRunner`, not direct `Command`. Rule-registry extensions later add
declarative safety-contract fields but do not create a second authorizer.
Rollback removes the `AuthorizedAction` path as a unit; do not restore a direct
executor-to-runner/trash path while retaining live-fingerprint tests, because
that would create a misleading partial security claim.
