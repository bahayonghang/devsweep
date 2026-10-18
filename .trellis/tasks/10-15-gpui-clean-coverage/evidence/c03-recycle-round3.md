# C03 real Recycle Bin measurement (round 3)

Date: 2026-10-18. CLI binary: `dist/ci-target/debug/devsweep.exe` (locked workspace build).
Isolated root: a fresh temp directory. `LOCALAPPDATA`, `APPDATA` and
`XDG_CONFIG_HOME` pointed at an empty temp home, so no real user cache,
journal or protection list was read or written. `HOME` was left untouched
because it is a read-only variable in this shell.

Fixture: a Python project (`pyproject.toml`) containing `.pytest_cache` with
a 4098-byte `payload.bin`.

## Flow and exit codes

| Step | Exit |
| --- | --- |
| `clean scan --root <fixture> --scope projects` | 0 |
| `clean plan --observation ... --select <id>` | 0 |
| `clean preview --plan ... --format json` | 0 |
| `clean execute --plan ... --preview-digest <digest> --confirm` | 0 |

Selected target: `python.pytest_cache:\\?\C:\Users\lyh\AppData\Local\Temp\c03-recycle-823973ca24\fixture\.pytest_cache`
Live preview digest: `sha256:6bcee2244ad34fcc217ec2aa6ee41fb1b56a96304860b980fa03cbadbee6ad9f`

## Recycle Bin observation

| Reading | Value |
| --- | --- |
| Recycle Bin item count before execute | 201 |
| Recycle Bin item count after execute | 202 |
| Source path still present after execute | False |
| Files remaining under the source path | 0 |

## Execute output

```
Execution completed.
python.pytest_cache:\\?\C:\Users\lyh\AppData\Local\Temp\c03-recycle-823973ca24\fixture\.pytest_cache | Move to Recycle Bin | Execution completed.
Recycle Bin moves remain recoverable until emptied. After owner commands complete, rescan to inspect remaining occupancy.

```

## Verdict

PASS: the artifact left its original path and the shell Recycle Bin item count rose by 1, so this is a real move-to-Recycle-Bin measurement, not a rename and not an in-place delete.

No file was permanently deleted, no real user cache was touched, and no
privileged or host-wide operation was performed. The fixture lived entirely in
a temp directory that was removed afterwards.
