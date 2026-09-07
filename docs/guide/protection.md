# Protect Paths

The persistent protection list records existing paths that DevSweep must not
clean. It is an extra safeguard in addition to built-in protection categories.
The old root `protect` is rejected; the command is nested under Clean.

```powershell
# The path must already exist. Mutations require --path and --confirm.
devsweep clean protect add --path C:\code\keep-me --confirm

# List protected paths.
devsweep clean protect list

# Remove an entry later.
devsweep clean protect remove --path C:\code\keep-me --confirm
```

On Windows, DevSweep stores the list below `%APPDATA%\devsweep` in
`protected-paths.json`. On other supported platforms it uses the corresponding
OS application-data location. Mutations also append a redacted
`protection_mutation` record to `%LOCALAPPDATA%\DevSweep\audit\v1\clean.jsonl`.
The payload stores a SHA-256 identity, never the raw path.

Paths are normalized for comparison. Removing an entry can still work after
the original path no longer exists. Adding a path requires it to exist so that
the stored protection is meaningful and canonical.

If the protection list cannot be loaded, DevSweep fails closed rather than
pretending no paths are protected.
