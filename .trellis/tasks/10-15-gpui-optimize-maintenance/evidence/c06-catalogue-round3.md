# C06 native CLI catalogue and refusal evidence (round 3)

Date: 2026-10-18. Binary: `dist/ci-target/debug/devsweep.exe` (locked workspace
build). All `LOCALAPPDATA`/`APPDATA`/`XDG_CONFIG_HOME` pointed at a fresh temp
directory, so no real user optimize journal was read or written. **No repair was
executed.** The session is not elevated, and no defrag/dism/w32tm command was
run.

## Catalogue (C06-AC1)

`optimize list --format json` exited 0 and returned 19 rows with an
`action_class` field:

| Class | Count | Ids |
| --- | --- | --- |
| execute | 7 | `dns.flush`, `maintenance.retrim`, `maintenance.clock_probe`, `maintenance.clock_sync`, `maintenance.image_scan`, `maintenance.image_repair`, `maintenance.dead_startup` |
| settings_handoff | 3 | `settings.storage_recommendations`, `settings.search`, `settings.energy_recommendations` |
| guidance | 8 | `guidance.drive_optimize`, `guidance.system_integrity`, `guidance.filesystem_check`, `guidance.network_reset`, `maintenance.update_cache`, `maintenance.icon_cache`, `maintenance.search`, `maintenance.winget_repair` |
| observe | 1 | `maintenance.disk_health` |

The eight v1 ids are all still present and their classes are unchanged. The v2
rows from the PRD are all present.

## Guidance has no mutation executor (C06-AC1, C06-R4)

`optimize plan --operation maintenance.search` wrote a plan whose body is
`{"version":1,"catalogue_version":2,"operation_id":"maintenance.search","action_class":"guidance"}`,
but `optimize preview --plan <that plan>` then exited **6** with:

```
invalid_optimize_selection: guidance entry maintenance.search is never dispatched
```

So a guidance row can be planned for display but can never be dispatched.

## Observe refuses planning (C06-R3)

`optimize plan --operation maintenance.disk_health` exited **6** with:

```
invalid_optimize_selection: observe entry maintenance.disk_health cannot be planned
```

`optimize health` (the read-only observe command) exited **0** and printed
`Disk health: 2 devices.` with `C: | unknown` and `D: | unknown`. The health
status is honestly reported as `unknown` rather than guessed.

## v1 execute row still previews (C06-AC1)

`optimize plan --operation dns.flush` exited 0 and `optimize preview --plan`
exited 0 with:

```
Optimize preview: dns.flush | Runs here. Digest: sha256:3deb8ab1399acf3f832ac4886c2e1b646ef3d7a9f6c2035283c4a8961d926b1d.
```

## Image scan does not start image repair (C06-R6)

`optimize plan --operation maintenance.image_scan` exited 0 and produced a plan
for `image_scan` only. It did not resolve or chain `maintenance.image_repair`.

## What this does NOT close

C06-AC4 asks for **disposable native Windows evidence of defrag/dism/w32tm**.
That remains **NOT_RUN**, and it is not approximated here:

- `defrag.exe /L`, `dism.exe /Online /Cleanup-Image /ScanHealth|RestoreHealth`
  and `w32tm.exe /resync` are host-level operations. They are not disposable,
  they can take a long time, and `/RestoreHealth` mutates the running image.
- The session is not elevated, and requesting elevation to run them on the
  productivity machine is exactly what C06-AC4 forbids ("no productivity-machine
  repairs are used as tests").

The fake-runner proof (`fake_runner_requery_omits_deleted_dead_startup_name`)
still covers side-effect and requery semantics without touching the host.
