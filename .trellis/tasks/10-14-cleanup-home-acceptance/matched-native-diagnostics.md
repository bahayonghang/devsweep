# Matched native process diagnostics

Both authorized 180-second diagnostic runs completed. All owned processes closed
normally, no forced termination or survivors remained, and fixture sentinels are
unchanged. These short, instrumented diagnostics do **not** replace the formal
548-second run or close its unexplained process-tree CPU anomaly.

## Exact comparison

| Input | Current production | Earlier candidate control |
| --- | --- | --- |
| Executable | `native-fixed/devsweep-desktop-production.exe` | `native/devsweep-desktop-production.exe` |
| SHA256 | `6904d6bb2d5204f76060226257957e7aa5ab9933c37bf5528efb5b7cd3e88feb` | `43c27388186df18794b1dfdf2a9c1f1c8fb1018764e2ea873bbb6b1d033a502b` |
| Raw directory | `native-diagnostic-current-01` | `native-diagnostic-old-01` |
| Host PID | 114992 | 94768 |
| Host FILETIME | 134364738257299738 | 134364740693692785 |
| Helper settle after exit | 285.49 ms | 244.47 ms |

All paths in the table are under `dist/cleanup-home-final/`. The 43c273 control
already contains the new Clean UI. It precedes the Windows Node script-argument
fix and final pure-production packaging; it is **not** the original pre-edit
264eed baseline. This comparison cannot isolate the entire UI redesign.

Both use `native-fixtures-02/manifest.json`, the same allowlisted environment,
new private application/WebView profiles, visible default window, unchanged OS
motion preference, no debug port and no user interaction. No scan, plan,
preview or execute is issued. `scripts/diagnose-native.py` reuses the unchanged
200 ms sampler, schedules normal close at 180 seconds, and reads command lines
only for PID/creation-identity-verified owned processes at 15, 125 and 170
seconds. The bounded CIM observations make these diagnostic runs distinct from
the uninstrumented formal run.

## Observations

Each run retains the same seven process identities through its three snapshots:
host, browser, crashpad handler, GPU process, NetworkService, StorageService and
renderer. Actual command lines identify WebView2 155.0.4283.45 and these roles.
Neither run creates the late eighth process seen in the formal run.

For a matching late interval of 128–178 seconds, the current run contains 244
samples and integrated tree CPU 0.03138% of one logical core, entirely attributed
to NetworkService at counter resolution. The earlier candidate contains 245
samples and 0.0% at counter resolution. Host, renderer and GPU process deltas
are 0.0% in both intervals. This interval is a diagnostic comparison, not five
formal retained windows and not a new performance threshold.

Machine-readable comparison:
`dist/cleanup-home-final/native-diagnostics-matched.json`. Each run's
`snapshot-{15,125,170}.json` retains exact command lines and process identities;
`measurement/{launch.json,samples.jsonl,summary.json}` retains the unchanged raw
sampler output. `diagnostic-complete.json` confirms sentinel/lifecycle checks.

## Boundary and remaining issue

The [formal native evidence](native-acceptance.md) remains authoritative:
host-PID RT-AC7 gates pass, while its stable full tree shows 70.08–82.49% of one
core. Almost all of that work belongs to PID 73556 / FILETIME
134364732016977045, which starts about 120.27 seconds after the host. The formal
run did not inspect command lines; that process exited normally before its role
could be captured. Its role cannot be recovered from the executable name alone.

The anomaly was not reproduced in these two controls and remains **OPEN and
unattributed**. This evidence does not prove a UI, WebView-runtime, operating-
system or external-service cause. It also does not close the unavailable native
debug-channel/dual-locale-preview evidence gap. The next targeted investigation
should capture the exact newly appearing child's command line and creation
identity while the high CPU is present, retaining any diagnostic instrumentation
separately from formal idle acceptance. No further blind reruns were performed.
