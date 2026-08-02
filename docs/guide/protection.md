# Protect Paths

The persistent protection list records existing paths that DevSweep must not
clean. It is an extra safeguard in addition to built-in protection categories.

```powershell
# The path must already exist.
cargo run --locked --bin devsweep -- protect add C:\code\keep-me

# List protected paths.
cargo run --locked --bin devsweep -- protect list

# Remove an entry later.
cargo run --locked --bin devsweep -- protect remove C:\code\keep-me
```

On Windows, DevSweep stores the list below `%APPDATA%\devsweep` in
`protected-paths.json`. On other supported platforms it uses the corresponding
OS application-data location.

Paths are normalized for comparison. Removing an entry can still work after
the original path no longer exists. Adding a path requires it to exist so that
the stored protection is meaningful and canonical.

If the protection list cannot be loaded, DevSweep fails closed rather than
pretending no paths are protected.
